//! Data models of modes.

use crate::expressions::tree::models::Expression;
use crate::lossless::models::LosslessToken;

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

/// A construct a native parser's tree cannot hold: its statement became a `Command` holding its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    /// A stable name for the construct, such as `tablesample` or `merge`.
    pub construct: &'static str,
    /// Byte range in the input where it starts and where parsing it ended.
    pub start: usize,
    pub end: usize,
}

/// Every statement a native dialect parser read, in order, and the gaps behind any `Command`.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub statements: Vec<Expression>,
    pub gaps: Vec<Gap>,
}
