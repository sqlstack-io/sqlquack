//! `databricks_profile`: how Databricks SQL is fingerprinted.

use sqlquack_syntax::tokens::models::Dialect;

use crate::databricks::profile::constants::{NAME_KEYWORDS, PAIRED_KEYWORDS};
use crate::dialects::main::is_generic_reserved::is_generic_reserved;
use crate::dialects::main::shared_texts::shared_texts;
use crate::dialects::models::FingerprintProfile;

/// Databricks SQL's fingerprint profile: the generic profile's DuckDB tokens and keywords, except that `ANY`, `SOME` and `ORDER` written as names keep their case.
pub(crate) fn databricks_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: |sql| shared_texts(sql, Dialect::DuckDB),
        is_reserved: |word| is_generic_reserved(word) && !NAME_KEYWORDS.contains(&word),
        folds_in_context: |uppers, index| {
            PAIRED_KEYWORDS.iter().any(|[word, next]| {
                uppers[index] == *word && uppers.get(index + 1).is_some_and(|after| after == next)
            })
        },
    }
}
