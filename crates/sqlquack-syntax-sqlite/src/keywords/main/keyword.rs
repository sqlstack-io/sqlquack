//! `keyword`: the keyword a word is, as `keywordCode` finds it.

use crate::keywords::constants::{FALLBACK_TOKENS, KEYWORDS};
use crate::keywords::models::Keyword;

/// The keyword `word` spells, compared without case, or none when it is an identifier.
pub fn keyword(word: &str) -> Option<Keyword> {
    KEYWORDS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(word))
        .map(|(_, token)| Keyword {
            token,
            fallback: FALLBACK_TOKENS.contains(token),
        })
}
