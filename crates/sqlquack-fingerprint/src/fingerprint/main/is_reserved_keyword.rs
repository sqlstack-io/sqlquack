//! `is_reserved_keyword`: whether a word can never be an unquoted identifier in a dialect.

use std::collections::HashSet;

use crate::fingerprint::_helpers::reserved_keywords::{DUCKDB, GENERIC, POSTGRES, SNOWFLAKE};
use crate::fingerprint::models::FingerprintDialect;

/// Whether the upper-cased `upper_word` is reserved in `dialect`; SQLite uses the generic list.
pub fn is_reserved_keyword(dialect: FingerprintDialect, upper_word: &str) -> bool {
    let keywords: &HashSet<String> = match dialect {
        FingerprintDialect::DuckDB => &DUCKDB,
        FingerprintDialect::PostgreSQL => &POSTGRES,
        FingerprintDialect::Snowflake => &SNOWFLAKE,
        FingerprintDialect::Generic | FingerprintDialect::SQLite => &GENERIC,
    };
    keywords.contains(upper_word)
}
