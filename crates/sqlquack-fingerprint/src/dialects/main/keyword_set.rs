//! `keyword_set`: a keyword list read into upper-cased words.

use std::collections::HashSet;

/// The upper-cased words of a keyword list, one per line, skipping blank lines and `#` comments.
pub(crate) fn keyword_set(data: &str) -> HashSet<String> {
    data.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_ascii_uppercase)
        .collect()
}
