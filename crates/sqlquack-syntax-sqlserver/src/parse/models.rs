//! What a parse gives.

use sqlquack_syntax::Expression;

/// A construct the shared tree cannot hold: its statement became a `Command` holding its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    /// A stable name for the construct, such as `cursor` or `xml_method`.
    pub construct: &'static str,
    /// Byte range in the input where it starts and where parsing it ended.
    pub start: usize,
    pub end: usize,
}

/// Where one statement was written, nested statements included, such as those of a `BEGIN ... END`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatementRange {
    /// Byte offset of the statement's first token.
    pub start: usize,
    /// Byte offset just past its last token, or past the `;` written right after it.
    pub end: usize,
    /// How many statements enclose it: 0 for a statement of the batch itself.
    pub depth: usize,
}

/// Every statement the parser read, in order, the gaps behind any `Command`, and where each statement was written.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub statements: Vec<Expression>,
    pub gaps: Vec<Gap>,
    /// Every statement read, nested ones included, in source order; the statements at depth 0 are `statements`.
    pub ranges: Vec<StatementRange>,
}
