//! `uncovered`: where the scanner left a byte outside every token, blank and comment.

use crate::scan::_helpers::scanner::Scanner;

/// The first byte of `sql` before any lexical fault that no token, blank or comment covers.
pub fn uncovered(sql: &str) -> Option<usize> {
    let (tokens, fault, skipped) = Scanner::new(sql).scan();
    let limit: usize = fault.map_or(sql.len(), |fault| fault.position);
    let mut ranges: Vec<(usize, usize)> = tokens
        .iter()
        .map(|token| (token.start, token.end))
        .chain(skipped)
        .collect();
    ranges.sort_unstable();
    let mut covered: usize = 0;
    for (start, end) in ranges {
        if start > covered && covered < limit {
            return Some(covered);
        }
        covered = covered.max(end);
    }
    (covered < limit).then_some(covered)
}
