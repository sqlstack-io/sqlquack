//! Reading SQL with Databricks' own scanner and parser, which give the formatter Databricks' tokens and trees.

use sqlquack_parse::databricks::scan::main::scan::scan;
use sqlquack_parse::databricks::scan::main::uncovered::uncovered;
use sqlquack_parse::databricks::scan::models::{Token, TokenKind};
use sqlquack_parse::databricks::{StrictFailure, parse_strict};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, Expression};

use crate::databricks::profile::constants::{
    COMMENT_CONTINUATION, HINT_FAILURE, LINE_CONTINUATION_FAILURE, OPAQUE_STATEMENT_FAILURE,
    TRY_CAST_OPERATOR,
};
use crate::dialects::main::gap_comments::gap_comments;
use crate::dialects::main::shared_type::shared_type;
use crate::dialects::models::{Comment, CommentSyntax, Lexeme, StatementSpan};
use crate::format::constants::{PARSE_FAILURE, TOKENIZE_FAILURE};

/// Databricks' comments: `--` to the end of the line, and `/* */`, which nest.
const DATABRICKS_COMMENTS: CommentSyntax = CommentSyntax {
    hash: false,
    nested: true,
};

/// The statements Databricks' grammar reads from `sql`, refused when any is kept as opaque text.
pub(crate) fn parse_databricks(sql: &str) -> Result<(Vec<Expression>, Vec<StatementSpan>), String> {
    let parsed = parse_strict(sql).map_err(|failure| match failure {
        StrictFailure::Parse(error) => format!("{PARSE_FAILURE}: {}", error.message),
        StrictFailure::Uncovered { position } => {
            format!("{PARSE_FAILURE}: no token covers byte {position}")
        }
    })?;
    if !parsed.gaps.is_empty() {
        return Err(OPAQUE_STATEMENT_FAILURE.to_string());
    }
    Ok((parsed.statements, Vec::new()))
}

/// The tokens and comments of `sql` as Databricks' scanner reads them; hints and a `--` comment continued past its line are refused.
pub(crate) fn lex_databricks(sql: &str) -> Result<(Vec<Lexeme>, Vec<Comment>), String> {
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
    if tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::HintStart | TokenKind::HintEnd))
    {
        return Err(HINT_FAILURE.to_string());
    }
    let starts = tokens.iter().map(|token| token.start).chain([sql.len()]);
    let ends = std::iter::once(0).chain(tokens.iter().map(|token| token.end));
    let comments: Vec<Comment> = ends
        .zip(starts)
        .flat_map(|(start, end)| gap_comments(sql, start, end, DATABRICKS_COMMENTS))
        .collect();
    if comments
        .iter()
        .any(|comment| comment.line && comment.text.ends_with(COMMENT_CONTINUATION))
    {
        return Err(LINE_CONTINUATION_FAILURE.to_string());
    }
    let lexemes: Vec<Lexeme> = tokens
        .iter()
        .map(|token| {
            let raw = &sql[token.start..token.end];
            Lexeme {
                start: token.start,
                end: token.end,
                kind: token_type(token, raw),
                raw: raw.to_string(),
            }
        })
        .collect();
    Ok((lexemes, comments))
}

/// The shared token type of a Databricks token: its literal kind, or what the shared tokenizer reads its text as.
fn token_type(token: &Token, raw: &str) -> TokenType {
    match token.kind {
        TokenKind::Quoted => TokenType::QuotedIdentifier,
        TokenKind::Number(_) => TokenType::Number,
        TokenKind::String { raw: true } => TokenType::RawString,
        TokenKind::String { raw: false } => TokenType::String,
        TokenKind::DollarString => TokenType::DollarString,
        TokenKind::Word => shared_type(raw, Dialect::PostgreSQL).unwrap_or(TokenType::Var),
        TokenKind::Op if raw == TRY_CAST_OPERATOR => TokenType::DColon,
        TokenKind::Op
        | TokenKind::Char(_)
        | TokenKind::HintStart
        | TokenKind::HintEnd
        | TokenKind::Fault(_) => {
            shared_type(raw, Dialect::PostgreSQL).unwrap_or(TokenType::Operator)
        }
    }
}
