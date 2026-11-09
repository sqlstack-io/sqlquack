//! Why an input does not parse.

/// Why the input does not parse, as SQL Server's parser would report it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the token it is at or near; the input's length at its end.
    pub position: usize,
    /// Whether the grammar or the lexer refused it, unlike a check the engine makes after parsing.
    pub syntax: bool,
}

/// Why strict parsing refused the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrictFailure {
    /// The parser rejected the input.
    Parse(ParseError),
}

impl StrictFailure {
    /// The parse error strict parsing refused.
    pub fn parse_error(&self) -> &ParseError {
        match self {
            Self::Parse(error) => error,
        }
    }
}
