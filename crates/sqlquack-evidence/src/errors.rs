//! Evidence failures.

use std::fmt::{self, Display};

/// Why evidence could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceError {
    /// A file could not be read or written.
    Io(String),
    /// Case files break the case format or the policy; every problem is listed.
    InvalidCases(Vec<String>),
    /// A stored result is malformed or does not match its name or contents.
    InvalidResult(String),
    /// DuckDB could not be started.
    DuckDb(String),
}

impl Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => formatter.write_str(message),
            Self::InvalidCases(problems) => {
                write!(formatter, "invalid case files: {}", problems.join("; "))
            }
            Self::InvalidResult(message) => write!(formatter, "invalid recorded result: {message}"),
            Self::DuckDb(message) => write!(formatter, "DuckDB failed: {message}"),
        }
    }
}

impl std::error::Error for EvidenceError {}
