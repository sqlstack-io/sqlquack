//! A Databricks recording journal's lines, and what importing them did.

use serde::Deserialize;

/// One statement of a Databricks recording journal, as the statement API answered it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalRow {
    /// The authored case's reference when it was recorded.
    pub key: String,
    /// The construct the case covers.
    pub construct: String,
    /// The suite the case belongs to.
    pub kind: String,
    /// The case hash the recording tool gave the case, which differs from SQLQuack's.
    pub case_hash: String,
    /// SHA-256 of the statement as sent, which names the case it answers.
    pub sql_sha256: String,
    /// SHA-256 of the statement ID.
    pub statement_id_sha256: String,
    /// When the answer arrived, in UTC, RFC 3339.
    pub recorded_at: String,
    /// `SUCCEEDED` or `FAILED`.
    pub state: String,
    /// The result columns, for succeeded statements.
    #[serde(default)]
    pub columns: Vec<JournalColumn>,
    /// The rows as the API wrote them: text cells, NULL as null.
    #[serde(default)]
    pub rows: Vec<Vec<Option<String>>>,
    /// The row count the API reported.
    #[serde(default)]
    pub row_count: Option<u64>,
    /// Whether the API returned fewer rows than the result held.
    #[serde(default)]
    pub truncated: bool,
    /// The error, for failed statements.
    #[serde(default)]
    pub error: Option<JournalFailure>,
}

/// One result column of a Databricks recording journal.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalColumn {
    /// The column name.
    pub name: String,
    /// The type's short name, such as `LONG`.
    pub type_name: String,
    /// The full type, such as `DECIMAL(10,2)`.
    pub type_text: String,
    /// Precision, for decimals.
    #[serde(default)]
    pub type_precision: Option<i64>,
    /// Scale, for decimals.
    #[serde(default)]
    pub type_scale: Option<i64>,
}

/// The error of a failed statement in a Databricks recording journal.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalFailure {
    /// The API's error code, such as `BAD_REQUEST`.
    pub error_code: Option<String>,
    /// The message, starting with the error class and holding the SQLSTATE.
    pub message: String,
}

/// What importing a Databricks recording journal did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportSummary {
    /// Results written with rows.
    pub rows: usize,
    /// Results written with a warehouse error.
    pub errors: usize,
    /// Statements whose case already has a result, left untouched.
    pub already_recorded: usize,
    /// Statements answering no pending case.
    pub unmatched: usize,
    /// Cases left pending, with why their statement's answer is not a case outcome.
    pub not_recordable: Vec<(String, String)>,
}

/// What importing one journal statement did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowImport {
    /// Wrote a result with rows.
    Rows,
    /// Wrote a result with a warehouse error.
    Error,
    /// The case already has a result, left untouched.
    AlreadyRecorded,
    /// The case stays pending: the statement's answer is not a case outcome, for this reason.
    NotRecordable(String),
}
