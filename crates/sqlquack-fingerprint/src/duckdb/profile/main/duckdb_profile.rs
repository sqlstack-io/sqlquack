//! `duckdb_profile`: how DuckDB SQL is fingerprinted.

use sqlquack_syntax::tokens::models::Dialect;

use crate::dialects::main::shared_texts::shared_texts;
use crate::dialects::models::FingerprintProfile;
use crate::duckdb::profile::_helpers::reserved_keywords::RESERVED_KEYWORDS;

/// DuckDB SQL's fingerprint profile: its tokens read by the shared tokenizer.
pub(crate) fn duckdb_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: |sql| shared_texts(sql, Dialect::DuckDB),
        is_reserved: |word| RESERVED_KEYWORDS.contains(word),
        folds_in_context: |_, _| false,
    }
}
