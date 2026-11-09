//! Where SQLite's name-capable `FOR` and `WITH` act as keywords.

use crate::sqlite::profile::constants::{CTE_NAME_FOLLOWERS, FOR_PARTNER, WITH_PARTNER};

/// Whether the upper-cased word at `index` acts as a keyword: `FOR` before `EACH`, and `WITH` before `RECURSIVE` or a common table expression's name.
pub(crate) fn acts_as_keyword(uppers: &[String], index: usize) -> bool {
    let next = uppers.get(index + 1).map(String::as_str);
    match uppers[index].as_str() {
        "FOR" => next == Some(FOR_PARTNER),
        "WITH" => {
            next == Some(WITH_PARTNER)
                || (next.is_some_and(starts_name)
                    && uppers
                        .get(index + 2)
                        .is_some_and(|after| CTE_NAME_FOLLOWERS.contains(&after.as_str())))
        }
        _ => false,
    }
}

/// Whether `token` is a name: an unquoted word or a quoted identifier.
fn starts_name(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || matches!(first, '_' | '"' | '`' | '['))
}
