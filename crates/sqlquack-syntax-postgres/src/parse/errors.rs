//! Why an input does not parse.

/// Why the input does not parse, as PostgreSQL would report it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the token it is at or near; the input's length at its end.
    pub position: usize,
    /// Whether PostgreSQL reports it with SQLSTATE 42601, unlike, say, 22025 or 54001.
    pub syntax: bool,
}

/// Why strict parsing refused the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrictFailure {
    /// The parser rejected the input, as PostgreSQL reports it.
    Parse(ParseError),
    /// The byte at `position` is in no token, space or comment.
    Uncovered { position: usize },
}

impl StrictFailure {
    /// The parse error, when strict parsing refused a parse error.
    pub fn parse_error(&self) -> Option<&ParseError> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Uncovered { .. } => None,
        }
    }
}
