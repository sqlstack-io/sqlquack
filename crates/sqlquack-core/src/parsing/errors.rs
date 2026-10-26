//! Parse failures.

use std::fmt::{self, Display};

use serde::Serialize;

use crate::parsing::models::SourceLocation;

/// SQL that could not be parsed, with the parser's message and position.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ParseFailure {
    /// The parser's message.
    pub message: String,
    /// Where the parser stopped, when it reported a position.
    pub location: Option<SourceLocation>,
}

impl Display for ParseFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.location {
            Some(location) => write!(
                formatter,
                "line {}, column {}: {}",
                location.line, location.column, self.message
            ),
            None => formatter.write_str(&self.message),
        }
    }
}

impl std::error::Error for ParseFailure {}
