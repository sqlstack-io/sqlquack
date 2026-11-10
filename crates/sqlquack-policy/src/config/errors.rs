//! Configuration failures.

use std::fmt::{self, Display};

/// Why a `sqlquack.toml` was rejected, with the file it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// The file.
    pub path: String,
    /// What is wrong with it.
    pub message: String,
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ConfigError {}
