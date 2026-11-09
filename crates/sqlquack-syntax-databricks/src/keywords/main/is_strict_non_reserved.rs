//! `is_strict_non_reserved`: whether a word is a strict-non-reserved keyword.

use crate::keywords::constants::STRICT_NON_RESERVED;

/// Whether `word` is a strict-non-reserved keyword: a name anywhere but never a table alias.
pub fn is_strict_non_reserved(word: &str) -> bool {
    STRICT_NON_RESERVED
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(word))
}
