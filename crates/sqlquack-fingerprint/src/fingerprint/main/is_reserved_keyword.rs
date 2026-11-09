//! `is_reserved_keyword`: whether a word can never be an unquoted identifier in a dialect.

use crate::dialects::main::profile::profile;
use crate::fingerprint::models::FingerprintDialect;

/// Whether the upper-cased `upper_word` is reserved in `dialect`; never, for a dialect built without its feature.
pub fn is_reserved_keyword(dialect: FingerprintDialect, upper_word: &str) -> bool {
    profile(dialect).is_some_and(|profile| (profile.is_reserved)(upper_word))
}
