//! Test case types for the DuckDB scanner.

use crate::scan::models::TokenKind;

/// An input and the tokens, by kind and value, that DuckDB's tokenizer splits it into.
pub(crate) struct TokensTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_tokens: &'static [(TokenKind, &'static str)],
}

/// A corpus file, relative to the crate, and the ids of its statements that fail to scan.
pub(crate) struct CorpusTestCase {
    pub(crate) description: &'static str,
    pub(crate) corpus: &'static str,
    pub(crate) expected_unscanned: &'static [&'static str],
}
