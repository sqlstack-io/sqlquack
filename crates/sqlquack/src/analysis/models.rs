//! Pipeline results.

use serde::Serialize;
use sqlquack_core::classification::models::Occurrence;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_core::reparse::models::FidelityOutcome;

/// What the pipeline found in one SQL input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Analysis {
    /// Whether the input parsed.
    pub parsed: bool,
    /// Statements parsed.
    pub statements: usize,
    /// Every construct used, in source order.
    pub occurrences: Vec<Occurrence>,
    /// The source print-back check for each statement.
    pub fidelity: Vec<FidelityOutcome>,
    /// Every refusal; the input converts only when this is empty.
    pub refusals: Vec<Refusal>,
}
