//! `keyword_status`: whether a word is a keyword, and whether it is reserved, compared without case.

use crate::keywords::constants::{RESERVED, UNRESERVED};
use crate::scan::models::KeywordStatus;

/// Whether `word` is one of SQL Server's keywords, and whether it is reserved; none for a name.
pub fn keyword_status(word: &str) -> Option<KeywordStatus> {
    let upper: String = word.to_ascii_uppercase();
    match (
        RESERVED.binary_search(&upper.as_str()),
        UNRESERVED.binary_search(&upper.as_str()),
    ) {
        (Ok(_index), _) => Some(KeywordStatus::Reserved),
        (Err(_not_reserved), Ok(_index)) => Some(KeywordStatus::Unreserved),
        (Err(_not_reserved), Err(_not_a_keyword)) => None,
    }
}
