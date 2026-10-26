//! Strict parsing for conversion and tolerant parsing, which never fails, for unfinished SQL.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::lossless::{tokenize_lossless, LosslessToken, TokenKind, TokenizeFailure};
use crate::tokens::TokenType;
use crate::{Dialect, Error, Expression, ParseOptions};

/// Why strict parsing refused the input.
#[derive(Debug, thiserror::Error)]
pub enum StrictFailure {
    /// The parser rejected the input.
    #[error(transparent)]
    Parse(#[from] Error),
    /// The input does not split into lossless tokens.
    #[error(transparent)]
    Tokens(#[from] TokenizeFailure),
}

/// The statements of `sql`, refusing on any parse error or any byte no token covers.
pub fn parse_strict(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, StrictFailure> {
    let statements = crate::parse_statements(sql, dialect, options)?;
    tokenize_lossless(sql, dialect)?;
    Ok(statements)
}

/// One statement's range and what tolerant parsing made of it.
#[derive(Debug)]
pub struct TolerantStatement {
    /// Byte offset of the statement's first byte, after any leading trivia.
    pub start: usize,
    /// Byte offset just past its last byte, before the `;` that ends it.
    pub end: usize,
    /// The parsed statement, or why it could not be parsed.
    pub parsed: Result<Expression, String>,
}

/// Everything tolerant parsing found in the input.
#[derive(Debug)]
pub struct TolerantParse {
    /// The lossless tokens; empty when the input could not be tokenized.
    pub tokens: Vec<LosslessToken>,
    /// Why the input could not be tokenized, if it could not.
    pub token_failure: Option<String>,
    /// Each statement between top-level semicolons, in order.
    pub statements: Vec<TolerantStatement>,
}

/// Parses each statement of `sql` on its own and keeps the ranges that do not parse.
pub fn parse_tolerant(sql: &str, dialect: Dialect, options: &ParseOptions) -> TolerantParse {
    let tokens = match catch_unwind(|| tokenize_lossless(sql, dialect)) {
        Ok(Ok(tokens)) => tokens,
        Ok(Err(failure)) => return unparsed_whole(sql, failure.to_string()),
        Err(_) => return unparsed_whole(sql, "the tokenizer panicked".to_owned()),
    };
    let statements: Vec<TolerantStatement> = statement_ranges(&tokens)
        .into_iter()
        .map(|(start, end)| TolerantStatement {
            start,
            end,
            parsed: parse_one_statement(&sql[start..end], dialect, options),
        })
        .collect();
    TolerantParse {
        tokens,
        token_failure: None,
        statements,
    }
}

fn unparsed_whole(sql: &str, reason: String) -> TolerantParse {
    let (start, end) = trimmed(sql);
    let statements = match start < end {
        true => vec![TolerantStatement {
            start,
            end,
            parsed: Err(reason.clone()),
        }],
        false => Vec::new(),
    };
    TolerantParse {
        tokens: Vec::new(),
        token_failure: Some(reason),
        statements,
    }
}

fn trimmed(sql: &str) -> (usize, usize) {
    let start = sql.len() - sql.trim_start().len();
    (start, sql.trim_end().len().max(start))
}

/// Byte ranges of the significant tokens between top-level semicolons.
fn statement_ranges(tokens: &[LosslessToken]) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut current: Option<(usize, usize)> = None;
    let mut depth = 0_usize;
    for token in tokens {
        let TokenKind::Significant(kind) = token.kind else {
            continue;
        };
        match kind {
            TokenType::LParen | TokenType::LBracket | TokenType::LBrace => depth += 1,
            TokenType::RParen | TokenType::RBracket | TokenType::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenType::Semicolon if depth == 0 => {
                ranges.extend(current.take());
                continue;
            }
            _ => {}
        }
        current = Some(match current {
            Some((start, _)) => (start, token.end),
            None => (token.start, token.end),
        });
    }
    ranges.extend(current);
    ranges
}

fn parse_one_statement(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Expression, String> {
    let parsed = catch_unwind(AssertUnwindSafe(|| {
        crate::parse_statements(sql, dialect, options)
    }));
    match parsed {
        Ok(Ok(mut statements)) if statements.len() == 1 => Ok(statements.remove(0)),
        Ok(Ok(statements)) => Err(format!(
            "expected one statement, found {}",
            statements.len()
        )),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("the parser panicked".to_owned()),
    }
}
