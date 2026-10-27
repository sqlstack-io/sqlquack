//! Modes: statements.

use std::panic::catch_unwind;
use std::panic::AssertUnwindSafe;

use crate::expressions::tree::models::Expression;
use crate::guard::models::ParseOptions;
use crate::lossless::models::LosslessToken;
use crate::lossless::models::TokenKind;
use crate::modes::models::TolerantParse;
use crate::modes::models::TolerantStatement;
use crate::tokens::models::Dialect;
use crate::tokens::models::TokenType;

pub(crate) fn unparsed_whole(sql: &str, reason: String) -> TolerantParse {
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

pub(crate) fn trimmed(sql: &str) -> (usize, usize) {
    let start = sql.len() - sql.trim_start().len();
    (start, sql.trim_end().len().max(start))
}

/// Byte ranges of the significant tokens between top-level semicolons.
pub(crate) fn statement_ranges(tokens: &[LosslessToken]) -> Vec<(usize, usize)> {
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

pub(crate) fn parse_one_statement(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Expression, String> {
    let parsed = catch_unwind(AssertUnwindSafe(|| {
        crate::parser::main::parse_statements::parse_statements(sql, dialect, options)
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
