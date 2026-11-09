//! `sqlite_profile`: how SQLite SQL is fingerprinted.

use sqlquack_syntax::tokens::models::Dialect;

use crate::dialects::main::is_generic_reserved::is_generic_reserved;
use crate::dialects::main::shared_texts::shared_texts;
use crate::dialects::models::FingerprintProfile;

/// SQLite SQL's fingerprint profile: its tokens read by the shared tokenizer, with the generic keywords.
pub(crate) fn sqlite_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: |sql| shared_texts(sql, Dialect::SQLite),
        is_reserved: is_generic_reserved,
    }
}
