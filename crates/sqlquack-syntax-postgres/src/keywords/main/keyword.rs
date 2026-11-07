//! `keyword`: the keyword a folded identifier spells, if any.

use crate::keywords::constants::KEYWORDS;
use crate::keywords::models::Keyword;

/// The keyword `folded` (already lower-cased) spells, if any.
pub fn keyword(folded: &str) -> Option<Keyword> {
    match KEYWORDS.binary_search_by(|keyword| keyword.name.cmp(folded)) {
        Ok(index) => KEYWORDS.get(index).copied(),
        Err(_not_a_keyword) => None,
    }
}
