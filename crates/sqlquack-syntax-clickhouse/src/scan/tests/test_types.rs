//! Test case types for the ClickHouse lexer.

use crate::scan::models::TokenKind;

/// An input and every token ClickHouse's own lexer makes of it: kind and byte range.
pub(crate) struct LexTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_tokens: Vec<(TokenKind, usize, usize)>,
}
