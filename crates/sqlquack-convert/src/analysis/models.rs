//! Pipeline results.

use serde::Serialize;
use sqlquack_core::classification::models::Occurrence;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_rules::conversion::models::Converted;

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
