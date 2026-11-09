//! `is_reserved`: whether a word is reserved, compared without case.

use crate::keywords::main::keyword_status::keyword_status;
use crate::scan::models::KeywordStatus;

/// Whether `word` is reserved, so it names nothing unless delimited.
pub fn is_reserved(word: &str) -> bool {
    keyword_status(word) == Some(KeywordStatus::Reserved)
}
