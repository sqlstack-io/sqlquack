//! Data models of modes.

use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::lossless::models::LosslessToken;

/// One statement's range and what tolerant parsing made of it.
#[derive(Debug)]
pub struct TolerantStatement {
    /// Byte offset of the statement's first byte, after any leading trivia.
    pub start: usize,
    /// Byte offset just past its last byte, before the `;` that ends it.
    pub end: usize,
    /// The parsed statement, or why it could not be parsed.
    pub parsed: Result<Expression, String>,
}

/// Everything tolerant parsing found in the input.
#[derive(Debug)]
pub struct TolerantParse {
    /// The lossless tokens; empty when the input could not be tokenized.
    pub tokens: Vec<LosslessToken>,
    /// Why the input could not be tokenized, if it could not.
    pub token_failure: Option<String>,
    /// Each statement between top-level semicolons, in order.
    pub statements: Vec<TolerantStatement>,
}
