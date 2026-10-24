//! Census failures.

use std::fmt::{self, Display};
use std::path::PathBuf;

/// Why a census could not run.
#[derive(Debug)]
pub enum CensusError {
    /// No `.sql` file was found under the given paths.
    NoInputs,
    /// A path could not be read.
    Read {
        /// The path.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
    /// The worker threads could not start.
    Workers(String),
}

impl Display for CensusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInputs => formatter.write_str("no .sql files found under the given paths"),
            Self::Read { path, message } => {
                write!(formatter, "cannot read {}: {message}", path.display())
            }
            Self::Workers(message) => write!(formatter, "cannot start census workers: {message}"),
        }
    }
}

impl std::error::Error for CensusError {}
