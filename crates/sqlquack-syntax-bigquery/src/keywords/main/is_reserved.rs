//! `is_reserved`: whether a word is one of GoogleSQL's reserved keywords.

use crate::keywords::constants::RESERVED;

/// Whether `word` is a reserved keyword, which only backquotes make a name.
pub fn is_reserved(word: &str) -> bool {
    RESERVED
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(word))
}
