//! Refusals and the decisions behind them.

use std::fmt::{self, Display};

use serde::Serialize;

use crate::classification::models::Construct;
use crate::parsing::models::SourceLocation;

/// Why a construct or statement is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalCategory {
    /// No verified rule converts the construct yet.
    MissingRule,
    /// DuckDB cannot express the construct's source meaning.
    NotExpressible,
    /// The SQL did not parse, or printing it back in its own dialect changed the tree.
    ParseFidelity,
}

impl RefusalCategory {
    /// The category as it appears in reports.
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingRule => "missing rule",
            Self::NotExpressible => "not expressible in DuckDB",
            Self::ParseFidelity => "parse fidelity",
        }
    }
}

impl Display for RefusalCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// One refusal, precise enough to act on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Refusal {
    /// The refused construct; absent when the whole statement is refused.
    pub construct: Option<Construct>,
    /// Where the refused construct or failure is.
    pub location: Option<SourceLocation>,
    /// Why it is refused.
    pub category: RefusalCategory,
    /// The specific reason, such as the differing tree path.
    pub reason: String,
}

/// What the rule catalogue says about one construct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// No rule covers the construct.
    Missing,
    /// The construct is declared impossible to express in DuckDB, for the given reason.
    NotExpressible {
        /// Why DuckDB cannot express it.
        reason: String,
    },
}
