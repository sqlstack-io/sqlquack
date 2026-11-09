//! `sqlite_profile`: how SQLite SQL is fingerprinted.

use sqlquack_syntax::tokens::models::Dialect;

use crate::dialects::main::is_generic_reserved::is_generic_reserved;
use crate::dialects::main::shared_texts::shared_texts;
use crate::dialects::models::FingerprintProfile;
use crate::sqlite::profile::_helpers::keyword_context::acts_as_keyword;
use crate::sqlite::profile::constants::NAME_KEYWORDS;

/// SQLite SQL's fingerprint profile: its tokens read by the shared tokenizer, with the generic keywords less those SQLite lets name things, which fold only where they act as keywords.
pub(crate) fn sqlite_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: |sql| shared_texts(sql, Dialect::SQLite),
        is_reserved: |word| is_generic_reserved(word) && !NAME_KEYWORDS.contains(&word),
        folds_in_context: acts_as_keyword,
    }
}
