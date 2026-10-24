//! Policy failures.

use std::fmt::{self, Display};

/// Why a policy file was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    /// The file could not be read.
    Read(String),
    /// The file is not valid TOML for the schema, including unknown keys.
    Schema(String),
    /// The file parsed but breaks a validation rule; every broken rule is listed.
    Invalid(Vec<String>),
}

impl Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(message) => write!(formatter, "cannot read the policy file: {message}"),
            Self::Schema(message) => {
                write!(formatter, "the policy file breaks its schema: {message}")
            }
            Self::Invalid(problems) => {
                write!(
                    formatter,
                    "the policy file is invalid: {}",
                    problems.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for PolicyError {}
