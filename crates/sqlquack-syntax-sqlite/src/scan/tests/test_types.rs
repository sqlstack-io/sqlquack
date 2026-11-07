//! Test case types for the SQLite scanner.

use crate::scan::models::TokenKind;

/// An input and the tokens, by kind and value, that SQLite's tokenizer splits it into.
pub(crate) struct TokensTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_tokens: &'static [(TokenKind, &'static str)],
}

/// An input and the first byte no token, blank or comment covers.
pub(crate) struct UncoveredTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_uncovered: Option<usize>,
}
