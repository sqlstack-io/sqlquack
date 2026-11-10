//! Pipeline results.

use std::collections::BTreeMap;

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

/// What the caller allows, confirms and refuses on top of the built-in defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConversionChoices {
    /// The configuration file help names, such as `sqlquack.toml`; none without one.
    pub config_file: Option<String>,
    /// The configuration's `[allow]`: known differences it allows (`true`) or refuses (`false`).
    pub allow: BTreeMap<String, bool>,
    /// Refuse every known difference.
    pub strict: bool,
    /// A Python runtime registers the project's Python functions.
    pub python_runtime: bool,
}
