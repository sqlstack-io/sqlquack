//! What a parse yields: the statements, the gaps that made a statement opaque, or the error.

use sqlquack_syntax::Expression;

/// Why the input does not parse, as PostgreSQL would report it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the token it is at or near; the input's length at its end.
    pub position: usize,
    /// Whether PostgreSQL reports it as a syntax error (SQLSTATE 42601); scanner errors such as an
    /// invalid Unicode escape (22025) and the stack depth limit (54001) are not.
    pub syntax: bool,
}

/// A construct the shared tree cannot hold: its statement became a `Command` holding its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    /// A stable name for the construct, such as `tablesample` or `merge`.
    pub construct: &'static str,
    /// Byte range in the input where it starts and where parsing it ended.
    pub start: usize,
    pub end: usize,
}

/// Every statement of the input, in order, and the gaps behind any `Command`.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub statements: Vec<Expression>,
    pub gaps: Vec<Gap>,
}
