//! `is_generic_reserved`: whether every dialect with a keyword list reserves a word.

use crate::dialects::_helpers::generic_keywords::GENERIC_KEYWORDS;

/// Whether the upper-cased `upper_word` is reserved in every dialect with a keyword list, as SQL in no named dialect and SQLite read it.
pub(crate) fn is_generic_reserved(upper_word: &str) -> bool {
    GENERIC_KEYWORDS.contains(upper_word)
}
