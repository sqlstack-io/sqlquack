//! Fingerprint failures.

use std::fmt::{self, Display};

/// SQL the dialect's tokenizer rejects, or a dialect name no tokenizer reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FingerprintError {
    /// The tokenizer stopped; the message names why.
    Tokenize(String),
    /// The PostgreSQL scanner returned a token outside the SQL, at this byte offset.
    TokenRange(usize),
    /// No fingerprint dialect has this name.
    UnknownDialect(String),
}

impl Display for FingerprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tokenize(message) => write!(formatter, "SQL tokenization failed: {message}"),
            Self::TokenRange(start) => write!(formatter, "SQL tokenization failed at byte {start}"),
            Self::UnknownDialect(name) => {
                write!(formatter, "unknown SQL fingerprint dialect '{name}'")
            }
        }
    }
}

impl std::error::Error for FingerprintError {}
