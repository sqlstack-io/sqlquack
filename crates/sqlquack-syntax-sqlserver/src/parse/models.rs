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

/// Every statement the parser read, in order, and the gaps behind any `Command`.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub statements: Vec<Expression>,
    pub gaps: Vec<Gap>,
}
