//! Why an input does not parse.

/// Why the input does not parse, as GoogleSQL's parser would report it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset of the token it is at or near; the input's length at its end.
    pub position: usize,
    /// Whether it is a syntax refusal (`Syntax error: ...`), not a check such as the nesting limit.
    pub syntax: bool,
}

/// Why strict parsing refused the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StrictFailure {
    /// The parser rejected the input.
    Parse(ParseError),
    /// The byte at `position` is in no token, blank or comment.
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
