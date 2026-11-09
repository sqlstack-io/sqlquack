//! The significant token texts of SQL under a dialect's tokenizer.

use sqlquack_syntax::lossless::models::TokenKind;
use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_syntax_bigquery::scan::main::scan::scan as bigquery_scan;
use sqlquack_syntax_bigquery::scan::models::TokenKind as BigQueryTokenKind;
use sqlquack_syntax_postgres::scan::main::scan::scan;
use sqlquack_syntax_postgres::scan::models::TokenKind as PostgresTokenKind;
use sqlquack_syntax_sqlserver::scan::main::scan::scan as sqlserver_scan;
use sqlquack_syntax_sqlserver::scan::models::TokenKind as SqlServerTokenKind;

use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::models::FingerprintDialect;

/// The text of each significant token of `sql`, in order: no whitespace and no comments.
pub(crate) fn significant_texts(
    sql: &str,
    dialect: FingerprintDialect,
) -> Result<Vec<&str>, FingerprintError> {
    let lossless: Dialect = match dialect {
        FingerprintDialect::PostgreSQL => return postgres_texts(sql),
        FingerprintDialect::BigQuery => return bigquery_texts(sql),
        FingerprintDialect::SqlServer => return sqlserver_texts(sql),
        FingerprintDialect::Generic | FingerprintDialect::DuckDB => Dialect::DuckDB,
        FingerprintDialect::Snowflake => Dialect::Snowflake,
        FingerprintDialect::SQLite => Dialect::SQLite,
    };
    Ok(sqlquack_syntax_snowflake::tokenize(sql, lossless)
        .map_err(|error| FingerprintError::Tokenize(error.to_string()))?
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Significant(_)))
        .map(|token| token.text(sql))
        .collect())
}

/// PostgreSQL's scanner tokens; a scanner fault fails the whole stream.
fn postgres_texts(sql: &str) -> Result<Vec<&str>, FingerprintError> {
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

/// BigQuery's scanner tokens; a scanner fault fails the whole stream.
fn bigquery_texts(sql: &str) -> Result<Vec<&str>, FingerprintError> {
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

/// SQL Server's scanner tokens; a scanner fault fails the whole stream.
fn sqlserver_texts(sql: &str) -> Result<Vec<&str>, FingerprintError> {
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
