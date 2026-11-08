//! GoogleSQL's own scanner and parser, which give the formatter BigQuery's tokens and trees.

use std::collections::HashMap;
use std::mem::{Discriminant, discriminant};

use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax_bigquery::keywords::main::is_reserved::is_reserved;
use sqlquack_syntax_bigquery::scan::main::scan::scan;
use sqlquack_syntax_bigquery::scan::main::uncovered::uncovered;
use sqlquack_syntax_bigquery::scan::models::{Token, TokenKind};
use sqlquack_syntax_bigquery::{StrictFailure, parse_strict};

use crate::format::_helpers::lexemes::{Comment, Lexeme};
use crate::format::_helpers::native::{CommentSyntax, gap_comments, shared_type};
use crate::format::constants::{
    BIGQUERY_OPAQUE_STATEMENT_FAILURE, PARSE_FAILURE, TOKENIZE_FAILURE,
};

/// GoogleSQL's comments: `#` and `--` to the end of the line, and `/* */`, which do not nest.
const GOOGLESQL_COMMENTS: CommentSyntax = CommentSyntax {
    hash: true,
    nested: false,
};

/// The statements GoogleSQL's grammar reads from `sql`, refused when any is kept as opaque text.
pub(crate) fn parse_bigquery(sql: &str) -> Result<Vec<Expression>, String> {
    let parsed = parse_strict(sql).map_err(|failure| match failure {
        StrictFailure::Parse(error) => format!("{PARSE_FAILURE}: {}", error.message),
        StrictFailure::Uncovered { position } => {
            format!("{PARSE_FAILURE}: no token covers byte {position}")
        }
    })?;
    if !parsed.gaps.is_empty() {
        return Err(BIGQUERY_OPAQUE_STATEMENT_FAILURE.to_string());
    }
    Ok(parsed.statements)
}

/// The tokens and comments of `sql` as GoogleSQL's scanner reads them; a word is a keyword when reserved or spelled as one.
pub(crate) fn lex_bigquery(sql: &str) -> Result<(Vec<Lexeme>, Vec<Comment>), String> {
    let tokens: Vec<Token> = scan(sql);
    if let Some(fault) = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::Fault(_)))
    {
        return Err(format!("{TOKENIZE_FAILURE}: {}", fault.value));
    }
    if let Some(position) = uncovered(sql) {
        return Err(format!(
            "{TOKENIZE_FAILURE}: no token covers byte {position}"
        ));
    }
    let starts = tokens.iter().map(|token| token.start).chain([sql.len()]);
    let ends = std::iter::once(0).chain(tokens.iter().map(|token| token.end));
    let comments: Vec<Comment> = ends
        .zip(starts)
        .flat_map(|(start, end)| gap_comments(sql, start, end, GOOGLESQL_COMMENTS))
        .collect();
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

/// The shared token type of a GoogleSQL token: its literal kind, or what the shared tokenizer reads its text as; a reserved word the shared tokenizer reads as a name is `Unknown`, which the guide treats as a keyword.
fn token_type(token: &Token, raw: &str) -> TokenType {
    let triple = |quote: &str| {
        raw.trim_start_matches(['r', 'R', 'b', 'B'])
            .starts_with(quote)
    };
    match token.kind {
        TokenKind::Quoted => TokenType::QuotedIdentifier,
        TokenKind::Number(_) => TokenType::Number,
        TokenKind::String { raw: true, .. } => TokenType::RawString,
        TokenKind::String { bytes: true, .. } => TokenType::ByteString,
        TokenKind::String { .. } if triple("'''") => TokenType::TripleSingleQuotedString,
        TokenKind::String { .. } if triple("\"\"\"") => TokenType::TripleDoubleQuotedString,
        TokenKind::String { .. } => TokenType::String,
        TokenKind::Word => match shared_type(raw) {
            Some(TokenType::Var) | None if is_reserved(raw) => TokenType::Unknown,
            shared => shared.unwrap_or(TokenType::Var),
        },
        TokenKind::Op | TokenKind::Char(_) if raw.starts_with('@') => TokenType::DAt,
        TokenKind::Op | TokenKind::Char(_) | TokenKind::Fault(_) => {
            shared_type(raw).unwrap_or(TokenType::Operator)
        }
    }
}
