//! `is_reserved_keyword`: whether a word can never be an unquoted identifier in a dialect.

use std::collections::HashSet;

use sqlquack_syntax_bigquery::keywords::main::is_reserved::is_reserved as is_bigquery_reserved;
use sqlquack_syntax_sqlserver::keywords::main::is_reserved::is_reserved as is_sqlserver_reserved;

use crate::fingerprint::_helpers::reserved_keywords::{DUCKDB, GENERIC, POSTGRES, SNOWFLAKE};
use crate::fingerprint::models::FingerprintDialect;

/// Whether the upper-cased `upper_word` is reserved in `dialect`; SQLite uses the generic list, and BigQuery and SQL Server their parsers' lists.
pub fn is_reserved_keyword(dialect: FingerprintDialect, upper_word: &str) -> bool {
    let keywords: &HashSet<String> = match dialect {
        FingerprintDialect::BigQuery => return is_bigquery_reserved(upper_word),
        FingerprintDialect::SqlServer => return is_sqlserver_reserved(upper_word),
        FingerprintDialect::DuckDB => &DUCKDB,
        FingerprintDialect::PostgreSQL => &POSTGRES,
        FingerprintDialect::Snowflake => &SNOWFLAKE,
        FingerprintDialect::Generic | FingerprintDialect::SQLite => &GENERIC,
    };
    keywords.contains(upper_word)
}
