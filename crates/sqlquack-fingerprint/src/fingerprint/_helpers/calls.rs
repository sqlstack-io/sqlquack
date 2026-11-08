//! Name positions, unquoted words and calls in an upper-cased token stream.

use crate::fingerprint::constants::{
    ALIAS_KEYWORD, CALL_CLOSER, CALL_OPENER, COLUMN_LIST_RELATION_KEYWORDS, CTE_BODY_OPENERS,
    NAME_PREFIXES, PLACEHOLDER_PREFIX,
};

/// For every upper-cased token, whether it follows `.`, `:`, `@`, `$` or `AS`.
pub(crate) fn name_positions(uppers: &[String]) -> Vec<bool> {
    let mut positions: Vec<bool> = Vec::with_capacity(uppers.len());
    let mut previous: Option<&str> = None;
    for upper in uppers {
        positions.push(
            previous.is_some_and(|word| word == ALIAS_KEYWORD || NAME_PREFIXES.contains(&word)),
        );
        previous = Some(upper.as_str());
    }
    positions
}

/// Whether `text` is an unquoted ASCII word whose letter case carries no meaning.
pub(crate) fn is_unquoted_word(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '$'))
}

/// The index of the `)` closing each `(`, by the index of the `(`.
pub(crate) fn matching_closers(uppers: &[String]) -> Vec<Option<usize>> {
    let mut closers: Vec<Option<usize>> = vec![None; uppers.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, token) in uppers.iter().enumerate() {
        if token == CALL_OPENER {
            open.push(index);
        } else if token == CALL_CLOSER
            && let Some(opener) = open.pop()
        {
            closers[opener] = Some(index);
        }
    }
    closers
}

/// Whether the word at `index` is called: followed by `(` and neither a placeholder nor a relation.
pub(crate) fn calls(raw: &str, uppers: &[String], closers: &[Option<usize>], index: usize) -> bool {
    if uppers.get(index + 1).is_none_or(|next| next != CALL_OPENER)
        || raw.starts_with(PLACEHOLDER_PREFIX)
    {
        return false;
    }
    let previous: Option<&str> = index.checked_sub(1).map(|before| uppers[before].as_str());
    if previous.is_some_and(|word| COLUMN_LIST_RELATION_KEYWORDS.contains(&word)) {
        return false;
    }
    let Some(closer) = closers[index + 1] else {
        return true;
    };
    !(uppers
        .get(closer + 1)
        .is_some_and(|next| next == ALIAS_KEYWORD)
        && uppers
            .get(closer + 2)
            .is_some_and(|next| CTE_BODY_OPENERS.contains(&next.as_str())))
}
