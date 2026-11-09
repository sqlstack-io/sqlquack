//! Pipeline results.

use crate::snowflake::conversion::models::Converted;
use serde::Serialize;
use sqlquack_diagnostics::classification::models::Occurrence;
use sqlquack_diagnostics::refusals::models::Refusal;

/// What the pipeline found in one SQL input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Analysis {
    /// Whether the input parsed.
    pub parsed: bool,
    /// Statements parsed.
    pub statements: usize,
    /// Every construct used, in source order.
    pub occurrences: Vec<Occurrence>,
    /// Every refusal; the input converts only when this is empty.
    pub refusals: Vec<Refusal>,
    /// The conversion of each statement that converted.
    pub conversions: Vec<Converted>,
}

/// What the caller confirms and refuses on top of the policy, and the policy file help names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversionChoices {
    /// The policy file, as help names it, such as `sqlquack.toml`.
    pub policy_file: String,
    /// Refuse every known divergence the policy refuses in strict mode.
    pub strict: bool,
    /// Both sessions run in UTC.
    pub utc_session: bool,
    /// The TIMESTAMP_TZ offsets the schema declares hold.
    pub confirm_declared_offsets: bool,
    /// A Python runtime registers the project's Python functions.
    pub python_runtime: bool,
    /// Refuse every known divergence that can return a different value rather than raise.
    pub loud_divergences_only: bool,
}
