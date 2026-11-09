//! SQL Server's own scanner's token texts.

use sqlquack_parse::sqlserver::scan::main::scan::scan as sqlserver_scan;
use sqlquack_parse::sqlserver::scan::models::TokenKind as SqlServerTokenKind;

use crate::fingerprint::errors::FingerprintError;

/// SQL Server's scanner tokens; a scanner fault fails the whole stream.
pub(crate) fn sqlserver_texts(sql: &str) -> Result<Vec<&str>, FingerprintError> {
    sqlserver_scan(sql)
        .into_iter()
        .map(|token| match token.kind {
            SqlServerTokenKind::Fault(_) => Err(FingerprintError::Tokenize(token.value)),
            _ => sql
                .get(token.start..token.end)
                .ok_or(FingerprintError::TokenRange(token.start)),
        })
        .collect()
}
