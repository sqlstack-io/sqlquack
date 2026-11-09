//! `generic_profile`: how SQL in no named dialect is fingerprinted.

use sqlquack_syntax::tokens::models::Dialect;

use crate::dialects::main::is_generic_reserved::is_generic_reserved;
use crate::dialects::main::shared_texts::shared_texts;
use crate::dialects::models::FingerprintProfile;

/// The fingerprint profile of SQL in no named dialect: DuckDB's tokens, with the keywords every dialect reserves.
pub(crate) fn generic_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: |sql| shared_texts(sql, Dialect::DuckDB),
        is_reserved: is_generic_reserved,
        folds_in_context: |_, _| false,
    }
}
