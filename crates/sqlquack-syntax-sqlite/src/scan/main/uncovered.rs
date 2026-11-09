//! `uncovered`: where the scanner left a byte outside every token, blank and comment.

use crate::scan::_helpers::scanner::Scanner;

/// The first byte no token, blank or comment covers: from a NUL byte, which ends SQLite's input.
pub fn uncovered(sql: &str) -> Option<usize> {
    let (tokens, skipped) = Scanner::new(sql).scan();
    let mut ranges: Vec<(usize, usize)> = tokens
        .iter()
        .map(|token| (token.start, token.end))
        .chain(skipped)
        .collect();
    ranges.sort_unstable();
    let mut covered: usize = 0;
    for (start, end) in ranges {
        if start > covered {
            return Some(covered);
        }
        covered = covered.max(end);
    }
    (covered < sql.len()).then_some(covered)
}
