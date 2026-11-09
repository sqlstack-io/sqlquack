//! Test case types for the SQL Server lexer.

use sqlquack_syntax::TokenKind as LosslessKind;

use crate::scan::models::KeywordStatus;

/// An input whose tokens must cover every byte in order, or leave `expected_uncovered` first.
pub(crate) struct CoverageTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_uncovered: Option<usize>,
}

/// An input and the significant token texts the engine splits it into.
pub(crate) struct BoundaryTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_texts: &'static [&'static str],
}

/// An input and the shared kinds of its significant tokens.
pub(crate) struct TokenTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_kinds: Vec<LosslessKind>,
}

/// An input and the undoubled values of its quoted runs.
pub(crate) struct QuotedTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_values: &'static [&'static str],
}

/// An input with an unclosed run and the engine's error number and message for it.
pub(crate) struct UnclosedTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_code: u32,
    pub(crate) expected_message: &'static str,
}

/// A word and its keyword status.
pub(crate) struct KeywordTestCase {
    pub(crate) description: &'static str,
    pub(crate) word: &'static str,
    pub(crate) expected_status: Option<KeywordStatus>,
}
