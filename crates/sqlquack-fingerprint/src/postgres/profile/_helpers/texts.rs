//! PostgreSQL's own scanner's token texts.

use sqlquack_parse::postgres::scan::main::scan::scan;
use sqlquack_parse::postgres::scan::models::TokenKind as PostgresTokenKind;

use crate::fingerprint::errors::FingerprintError;

/// PostgreSQL's scanner tokens; a scanner fault fails the whole stream.
pub(crate) fn postgres_texts(sql: &str) -> Result<Vec<&str>, FingerprintError> {
    scan(sql)
        .into_iter()
        .map(|token| match token.kind {
            PostgresTokenKind::Fault(_) => Err(FingerprintError::Tokenize(token.value)),
            _ => sql
                .get(token.start..token.end)
                .ok_or(FingerprintError::TokenRange(token.start)),
        })
        .collect()
}
