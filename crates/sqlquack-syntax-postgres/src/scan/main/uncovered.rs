//! `uncovered`: where the scanner skipped a byte outside every token, space and comment.

use crate::scan::_helpers::scanner::Scanner;

/// The first byte of `sql` before any lexical fault that no token, space or comment covers.
pub fn uncovered(sql: &str) -> Option<usize> {
    Scanner::new(sql).uncovered()
}
