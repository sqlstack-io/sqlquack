//! Shared helpers of the SQL Server lexer tests.

use sqlquack_syntax::{LosslessToken, TokenKind as LosslessKind};

use crate::scan::main::scan::scan;
use crate::scan::main::tokenize::tokenize;
use crate::scan::models::TokenKind;

/// The significant tokens' texts of `sql`.
pub(crate) fn texts(sql: &str) -> Vec<&str> {
    tokenize(sql)
        .expect("a tokenizable input")
        .into_iter()
        .filter(|token| matches!(token.kind, LosslessKind::Significant(_)))
        .filter_map(|token| sql.get(token.start..token.end))
        .collect()
}

/// The significant tokens' shared kinds of `sql`.
pub(crate) fn significant_kinds(sql: &str) -> Vec<LosslessKind> {
    tokenize(sql)
        .expect("a tokenizable input")
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| matches!(kind, LosslessKind::Significant(_)))
        .collect()
}

/// The first byte of `sql` its tokens skip, overlap or leave uncovered at the end, if any.
pub(crate) fn first_uncovered(sql: &str) -> Option<usize> {
    let tokens: Vec<LosslessToken> = tokenize(sql).expect("a tokenizable input");
    tokens
        .iter()
        .try_fold(0, |at: usize, token: &LosslessToken| {
            (token.start == at && token.end > token.start)
                .then_some(token.end)
                .ok_or(at)
        })
        .map_or_else(Some, |end: usize| (end != sql.len()).then_some(end))
}

/// The undoubled values of `sql`'s delimited names and strings.
pub(crate) fn quoted_values(sql: &str) -> Vec<String> {
    scan(sql)
        .into_iter()
        .filter(|token| matches!(token.kind, TokenKind::Quoted(_) | TokenKind::String { .. }))
        .map(|token| token.value)
        .collect()
}
