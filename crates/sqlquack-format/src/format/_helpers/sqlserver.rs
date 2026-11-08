//! SQL Server's own Transact-SQL scanner and parser, which give the formatter SQL Server's tokens and trees.

use std::collections::HashMap;
use std::mem::{Discriminant, discriminant};

use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax_sqlserver::scan::models::{NumberKind, Token, TokenKind};
use sqlquack_syntax_sqlserver::{StrictFailure, parse_strict, scan};

use crate::format::_helpers::lexemes::{Comment, Lexeme};
use crate::format::_helpers::native::{CommentSyntax, gap_comments, shared_type};
use crate::format::constants::{
    BATCH_SEPARATOR, BATCH_SEPARATOR_FAILURE, PARSE_FAILURE, SQLSERVER_OPAQUE_STATEMENT_FAILURE,
    TOKENIZE_FAILURE,
};

/// Transact-SQL's comments: `--` to the end of the line, and `/* */`, which nest.
const TRANSACT_SQL_COMMENTS: CommentSyntax = CommentSyntax {
    hash: false,
    nested: true,
};

/// The statements Transact-SQL's grammar reads from `sql`, refused when any is kept as opaque text.
pub(crate) fn parse_sqlserver(sql: &str) -> Result<Vec<Expression>, String> {
    let parsed = parse_strict(sql).map_err(|failure| match failure {
        StrictFailure::Parse(error) => format!("{PARSE_FAILURE}: {}", error.message),
    })?;
    if !parsed.gaps.is_empty() {
        return Err(SQLSERVER_OPAQUE_STATEMENT_FAILURE.to_string());
    }
    Ok(parsed.statements)
}

/// The tokens and comments of `sql` as Transact-SQL's scanner reads them; a `GO` batch separator is refused.
pub(crate) fn lex_sqlserver(sql: &str) -> Result<(Vec<Lexeme>, Vec<Comment>), String> {
    let tokens: Vec<Token> = scan(sql);
    if let Some(fault) = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Fault(_)))
    {
        return Err(format!("{TOKENIZE_FAILURE}: {}", fault.value));
    }
    let starts = tokens.iter().map(|token| token.start).chain([sql.len()]);
    let ends = std::iter::once(0).chain(tokens.iter().map(|token| token.end));
    let comments: Vec<Comment> = ends
        .zip(starts)
        .flat_map(|(start, end)| gap_comments(sql, start, end, TRANSACT_SQL_COMMENTS))
        .collect();
    if let Some(position) = uncovered(sql, &tokens, &comments) {
        return Err(format!(
            "{TOKENIZE_FAILURE}: no token covers byte {position}"
        ));
    }
    if (0..tokens.len()).any(|position| batch_separator(sql, &tokens, position)) {
        return Err(BATCH_SEPARATOR_FAILURE.to_string());
    }
    let mut kinds: HashMap<(&str, Discriminant<TokenKind>), TokenType> = HashMap::new();
    let mut lexemes: Vec<Lexeme> = Vec::with_capacity(tokens.len());
    for token in &tokens {
        let raw = &sql[token.start..token.end];
        let kind = *kinds
            .entry((raw, discriminant(&token.kind)))
            .or_insert_with(|| token_type(token, raw));
        lexemes.push(Lexeme {
            start: token.start,
            end: token.end,
            kind,
            raw: raw.to_string(),
        });
    }
    Ok((lexemes, comments))
}

/// The first byte the scanner skipped that is neither whitespace nor inside a comment, which a layout would drop.
fn uncovered(sql: &str, tokens: &[Token], comments: &[Comment]) -> Option<usize> {
    sql.char_indices()
        .find(|&(at, character)| {
            let token = tokens.partition_point(|token| token.end <= at);
            let comment = comments.partition_point(|comment| comment.end <= at);
            !character.is_whitespace()
                && !tokens.get(token).is_some_and(|token| token.start <= at)
                && !comments
                    .get(comment)
                    .is_some_and(|comment| comment.start <= at)
        })
        .map(|(at, _)| at)
}

/// Whether token `position` is a `GO` alone on its line or followed only by a repeat count, which a client reads as a batch separator, not SQL.
fn batch_separator(sql: &str, tokens: &[Token], position: usize) -> bool {
    let token = &tokens[position];
    if token.kind != TokenKind::Word || !token.value.eq_ignore_ascii_case(BATCH_SEPARATOR) {
        return false;
    }
    let line_start = sql[..token.start].rfind('\n').map_or(0, |at| at + 1);
    let line_end = sql[token.end..]
        .find('\n')
        .map_or(sql.len(), |at| token.end + at);
    let on_line = |offset: usize| {
        tokens
            .get(offset)
            .is_some_and(|other| other.start < line_end)
    };
    let after_previous = position == 0 || tokens[position - 1].end <= line_start;
    let count = tokens.get(position + 1).is_some_and(|next| {
        next.kind == TokenKind::Number(NumberKind::Integer) && next.start < line_end
    });
    after_previous && (!on_line(position + 1) || (count && !on_line(position + 2)))
}

/// The shared token type of a Transact-SQL token: its literal kind, or what the shared tokenizer reads its text as.
fn token_type(token: &Token, raw: &str) -> TokenType {
    match token.kind {
        TokenKind::Quoted(_) => TokenType::QuotedIdentifier,
        TokenKind::Variable => TokenType::Parameter,
        TokenKind::String { national: true } => TokenType::NationalString,
        TokenKind::String { national: false } => TokenType::String,
        TokenKind::Number(NumberKind::Binary) => TokenType::HexString,
        TokenKind::Number(_) => TokenType::Number,
        TokenKind::Word => shared_type(raw).unwrap_or(TokenType::Var),
        TokenKind::Op | TokenKind::Char(_) | TokenKind::Fault(_) => {
            shared_type(raw).unwrap_or(TokenType::Operator)
        }
    }
}
