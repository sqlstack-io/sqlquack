//! BigQuery's own scanner's token texts.

use sqlquack_parse::bigquery::scan::main::scan::scan as bigquery_scan;
use sqlquack_parse::bigquery::scan::models::TokenKind as BigQueryTokenKind;

use crate::fingerprint::errors::FingerprintError;

/// BigQuery's scanner tokens; a scanner fault fails the whole stream.
pub(crate) fn bigquery_texts(sql: &str) -> Result<Vec<&str>, FingerprintError> {
    bigquery_scan(sql)
        .into_iter()
        .map(|token| match token.kind {
            BigQueryTokenKind::Fault(_) => Err(FingerprintError::Tokenize(token.value)),
            _ => sql
                .get(token.start..token.end)
                .ok_or(FingerprintError::TokenRange(token.start)),
        })
        .collect()
}
