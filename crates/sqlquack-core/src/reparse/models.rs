//! Re-parse check results.

use std::fmt::{self, Display};

use serde::Serialize;

use crate::comparison::models::TreeDifference;

/// What printing a parsed statement back in its own dialect did to its tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FidelityOutcome {
    /// The printed SQL parses to the same tree.
    Identical,
    /// The first print changed the tree, which then settles; accepted only with a recorded proof.
    Canonicalised {
        /// Where the first print changed the tree.
        difference: TreeDifference,
    },
    /// The tree keeps changing on every print.
    Unstable {
        /// Where the second print changed the tree again.
        difference: TreeDifference,
    },
    /// The tree could not be printed.
    PrintFailed {
        /// The printer's message.
        message: String,
    },
    /// The printed SQL did not parse back to one statement.
    ReparseFailed {
        /// The parser's message.
        message: String,
    },
}

/// Why DuckDB SQL printed from an intended tree cannot be trusted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "failure", rename_all = "snake_case")]
pub enum PrinterCheckFailure {
    /// The intended tree could not be printed as DuckDB SQL.
    PrintFailed {
        /// The printer's message.
        message: String,
    },
    /// The printed SQL did not parse back as one DuckDB statement.
    ReparseFailed {
        /// The printed SQL.
        sql: String,
        /// The parser's message.
        message: String,
    },
    /// The printed SQL parses to a different tree than the one intended.
    Changed {
        /// The printed SQL.
        sql: String,
        /// Where the re-parsed tree differs from the intended one.
        difference: TreeDifference,
    },
}

impl Display for PrinterCheckFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrintFailed { message } => {
                write!(formatter, "the DuckDB printer failed: {message}")
            }
            Self::ReparseFailed { sql, message } => {
                write!(
                    formatter,
                    "printed DuckDB SQL `{sql}` does not parse: {message}"
                )
            }
            Self::Changed { sql, difference } => write!(
                formatter,
                "printed DuckDB SQL `{sql}` does not parse to the intended tree: {difference}"
            ),
        }
    }
}

impl std::error::Error for PrinterCheckFailure {}
