//! One recorded statement's translation outcome and the constructs it uses.

use serde::Serialize;

/// What one recorded statement shows about its translation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Converted, and DuckDB returned Snowflake's recorded rows.
    Matching,
    /// Converted, and DuckDB raised the same kind of error Snowflake recorded.
    MatchingError,
    /// Only column types were recorded, or only Snowflake ran both spellings: no DuckDB result.
    ShapeOnly,
    /// Converted, and DuckDB raised a declared divergence's guard where Snowflake did not.
    DeclaredDivergence,
    Refused,
    /// Refused, and Snowflake refused to compile it too, as recorded and as the case declares.
    MatchingRejection,
    /// The recording cannot show whether the replay is right.
    Unproven,
    /// No recorded result; the statement converts.
    UnrecordedTranslates,
    /// No recorded result; the statement is refused.
    UnrecordedRefused,
    /// The replay differed from the recording, or the evidence is invalid: a bug.
    Differs,
}

/// One statement's line in the file.
#[derive(Debug, Serialize)]
pub struct Statement {
    pub reference: String,
    pub hash: String,
    pub outcome: Outcome,
    /// The diagnostic codes of its refusals, or `divergence:<id>` for a guard's raise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
    /// The refused construct keys.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<String>,
    /// Every construct the Snowflake statement uses, which links it to catalogue forms.
    pub constructs: Vec<String>,
}

/// The whole file `sqlquack verify --translation-statements` writes.
#[derive(Debug, Serialize)]
pub struct StatementsFile {
    pub format: &'static str,
    pub statements: Vec<Statement>,
}
