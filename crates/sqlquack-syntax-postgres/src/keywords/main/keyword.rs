//! `keyword`: the keyword a folded identifier spells, if any.

use crate::keywords::constants::KEYWORDS;
use crate::keywords::models::Keyword;

/// The keyword `folded` (already lower-cased) spells, if any.
pub fn keyword(folded: &str) -> Option<Keyword> {
    KEYWORDS
        .binary_search_by(|keyword| keyword.name.cmp(folded))
        .ok()
        .and_then(|index| KEYWORDS.get(index).copied())
}
