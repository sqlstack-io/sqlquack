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
