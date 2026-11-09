//! Why an input does not parse.

/// Why the input does not parse, as ClickHouse's parser would report it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the token it is at; the input's length at its end.
    pub position: usize,
    /// Whether it is a syntax refusal (`SYNTAX_ERROR`, code 62), not a parser check.
    pub syntax: bool,
}
