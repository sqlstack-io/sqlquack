//! Refusals and the decisions behind them.

use std::fmt::{self, Display};

use serde::Serialize;

use crate::classification::models::Construct;
use crate::refusals::constants::NEEDS_DIVERGENCE;
use sqlquack_syntax::tokens::models::SourceLocation;

/// Why a construct or statement is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalCategory {
    /// No verified rule converts the construct yet.
    MissingRule,
    /// DuckDB cannot express the construct's source meaning.
    NotExpressible,
    /// The SQL did not parse.
    ParseFidelity,
    /// A rule covers the construct, but the operand types fall outside its proven conditions.
    TypeCondition,
    /// A relation or column name does not resolve, or resolves ambiguously.
    UnresolvedName,
    /// Converting the statement took longer, or emitted more SQL, than the conversion budget.
    ConversionBudget,
    /// Snowflake parses the statement but rejects it, so there is nothing to convert.
    SnowflakeRejects,
    /// The conversion needs a known divergence the policy refuses in this mode.
    DivergenceRefused,
}

impl RefusalCategory {
    /// The category as it appears in reports.
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingRule => "missing rule",
            Self::NotExpressible => "not expressible in DuckDB",
            Self::ParseFidelity => "parse fidelity",
            Self::TypeCondition => "outside a rule's type conditions",
            Self::UnresolvedName => "unresolved name",
            Self::ConversionBudget => "conversion budget exceeded",
            Self::SnowflakeRejects => "rejected by Snowflake",
            Self::DivergenceRefused => "a known divergence the policy refuses",
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

impl Refusal {
    /// The id of the known divergence a refused divergence needs; none for other refusals.
    pub fn divergence(&self) -> Option<&str> {
        if self.category != RefusalCategory::DivergenceRefused {
            return None;
        }
        let (_, rest) = self.reason.split_once(NEEDS_DIVERGENCE)?;
        rest.split_once('`').map(|(id, _)| id)
    }
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
