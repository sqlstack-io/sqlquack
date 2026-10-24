//! Result metadata, as the recorder writes it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlquack_compare::comparison::models::RecordedResult;
use sqlquack_policy::policy::models::SessionValue;

/// The metadata file beside each recorded result. Unknown keys are rejected.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResultMetadata {
    /// The evidence format.
    pub format: u32,
    /// The case hash; also the file name.
    pub case_hash: String,
    /// Whether the warehouse returned rows or an error.
    pub outcome: RecordedOutcome,
    /// The recorder name and version.
    pub recorder: String,
    /// The warehouse client name and version.
    pub connector: String,
    /// SHA-256 of the warehouse query ID, as lower-case hex; the ID itself is never stored.
    pub query_id_sha256: Option<String>,
    /// The warehouse version.
    pub warehouse_version: String,
    /// The DuckDB version the case was prepared against.
    pub duckdb_version: String,
    /// When the result was recorded, in UTC, RFC 3339.
    pub recorded_at: String,
    /// The session parameters the result was recorded under.
    pub session_parameters: BTreeMap<String, SessionValue>,
    /// The result columns as the warehouse described them.
    pub columns: Vec<WarehouseColumn>,
    /// Rows returned.
    pub row_count: Option<u64>,
    /// SHA-256 of the Arrow IPC file, as lower-case hex.
    pub arrow_sha256: Option<String>,
    /// The warehouse error, for error outcomes.
    pub error: Option<RecordedFailure>,
}

/// Whether the warehouse returned rows or an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordedOutcome {
    /// Rows, in the Arrow file.
    Rows,
    /// An error; there is no Arrow file.
    Error,
}

/// One result column as the Snowflake connector describes it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WarehouseColumn {
    /// The column name.
    pub name: String,
    /// The connector's numeric type code.
    pub type_code: i64,
    /// The connector's type name, such as `FIXED` or `TEXT`.
    pub type_name: String,
    /// Precision, for numbers.
    pub precision: Option<i64>,
    /// Scale, for numbers and time types.
    pub scale: Option<i64>,
    /// Length, for text and binary.
    pub internal_size: Option<i64>,
    /// Whether NULL is allowed.
    pub nullable: Option<bool>,
}

/// A warehouse error recorded as the case outcome.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedFailure {
    /// The warehouse error number.
    pub errno: Option<i64>,
    /// The SQLSTATE.
    pub sqlstate: Option<String>,
}

/// A loaded, integrity-checked result.
#[derive(Clone, Debug)]
pub struct StoredResult {
    /// Its metadata.
    pub metadata: ResultMetadata,
    /// Its rows, absent for error outcomes.
    pub rows: Option<RecordedResult>,
}
