//! The case file format and loaded cases.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlquack_compare::comparison::models::RowOrder;
use sqlquack_policy::policy::models::SessionValue;

/// One authored case file: shared settings and its cases.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFile {
    /// The edge-case category whose required inputs the file's cases must cover.
    pub category: String,
    /// The construct the cases exercise, such as `fn:CONCAT`.
    pub construct: String,
    /// The session profile the cases are recorded and replayed under.
    pub session_profile: String,
    /// The spelling pair the file proves; its cases then state a second, `canonical` spelling.
    #[serde(default)]
    pub equivalence: Option<String>,
    /// What the file's cases are evidence for, unless a case says otherwise; `conversion` by default.
    #[serde(default)]
    pub evidence: Option<EvidenceKind>,
    /// The cases.
    #[serde(rename = "case")]
    pub cases: Vec<CaseDefinition>,
}

/// One case: typed input rows and an expression or query over them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseDefinition {
    /// Unique within the file.
    pub id: String,
    /// What the case checks.
    pub description: String,
    /// Input columns and their source-dialect types.
    pub columns: Vec<InputColumn>,
    /// Input rows; each cell is a source-dialect literal such as `'a'`, `1.5` or `NULL`.
    pub rows: Vec<Vec<String>>,
    /// An expression over the input columns, evaluated per row in input order.
    #[serde(default)]
    pub expression: Option<String>,
    /// A query over the `input` relation.
    #[serde(default)]
    pub query: Option<String>,
    /// In equivalence files: the canonical spelling of `expression` or `query`.
    #[serde(default)]
    pub canonical: Option<String>,
    /// For queries: `ordered` when the query fixes the row order, else `bag`.
    #[serde(default)]
    pub order: Option<QueryOrder>,
    /// The edge-case inputs the case covers.
    pub tags: Vec<String>,
    /// Where an imported case comes from, such as a third-party test suite; never hashed.
    #[serde(default)]
    pub origin: Option<String>,
    /// sqlglot's expected DuckDB spelling, kept for reference only; never hashed or asserted.
    #[serde(default)]
    pub sqlglot_duckdb: Option<String>,
    /// A policy divergence the case demonstrates: Snowflake returns rows, DuckDB raises; never hashed.
    #[serde(default)]
    pub expected_divergence: Option<String>,
    /// What the case is evidence for, overriding its file's `evidence`.
    #[serde(default)]
    pub evidence: Option<EvidenceKind>,
    /// `false` records a typing case's column types and row count but no values; hashed when false.
    #[serde(default)]
    pub values: Option<bool>,
    /// Snowflake is expected to refuse to compile the SQL, past the recorder's check; never hashed.
    #[serde(default)]
    pub compile_error: bool,
    /// SQL project functions the query calls, created before it; their definitions are hashed.
    #[serde(default, rename = "function")]
    pub functions: Vec<CaseFunction>,
    /// User overrides conversion uses, proven by the case's recorded result; never hashed.
    #[serde(default, rename = "override")]
    pub overrides: Vec<CaseOverride>,
}

/// A user override a case converts with: its DuckDB SQL stands in for a Snowflake function.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseOverride {
    /// The override's name, a lower-case identifier.
    pub name: String,
    /// Its file's text: a FUNCTION() header with `replaces` and `dialect duckdb`, then the body.
    pub definition: String,
}

/// A SQL project function a case defines and calls.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseFunction {
    /// The function name, a lower-case identifier.
    pub name: String,
    /// The parameters, in order.
    #[serde(default)]
    pub arguments: Vec<InputColumn>,
    /// The scalar return type; exactly one of `returns` and `returns_table`.
    #[serde(default)]
    pub returns: Option<String>,
    /// The columns of a table function.
    #[serde(default)]
    pub returns_table: Option<Vec<InputColumn>>,
    /// The body: a scalar function's expression, a table function's query, or Python source.
    pub body: String,
    /// The body's language; SQL by default.
    #[serde(default)]
    pub language: FunctionBodyLanguage,
    /// A Python function's handler, a function the body defines.
    #[serde(default)]
    pub handler: Option<String>,
    /// A Python function's Snowflake runtime version, such as `3.12`.
    #[serde(default)]
    pub runtime_version: Option<String>,
}

/// The language a case function's body is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionBodyLanguage {
    /// A SQL expression or query.
    #[default]
    Sql,
    /// Python source defining the handler.
    Python,
}

/// What a case is evidence for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Values: the conversion replays on DuckDB with the recorded result.
    Conversion,
    /// Types only: the recorded column types and nullability, never compared with DuckDB.
    Typing,
}

/// An input column.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputColumn {
    /// The column name, a lower-case identifier.
    pub name: String,
    /// The source-dialect type.
    #[serde(rename = "type")]
    pub data_type: String,
}

/// How a query's rows are ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOrder {
    /// The query's ORDER BY fixes the order.
    Ordered,
    /// Rows compare as a multiset.
    Bag,
}

/// A case ready to record or verify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedCase {
    /// `<file>#<id>`, relative to the cases directory.
    pub reference: String,
    /// What the case checks.
    pub description: String,
    /// The category of its file.
    pub category: String,
    /// The construct of its file.
    pub construct: String,
    /// The session profile name.
    pub session_profile: String,
    /// The Snowflake session parameters it records under.
    pub session_parameters: BTreeMap<String, SessionValue>,
    /// Its profile's pinned parameters, set while recording but not hashed.
    pub pinned_parameters: BTreeMap<String, SessionValue>,
    /// The full source SQL that is recorded.
    pub source_sql: String,
    /// How rows compare.
    pub order: RowOrder,
    /// The content hash naming its result.
    pub hash: String,
    /// What the case proves.
    pub role: CaseRole,
    /// The policy divergence the case demonstrates, if any.
    pub expected_divergence: Option<String>,
    /// Whether the recording keeps the result's values, not only its column types and row count.
    pub values: bool,
    /// Whether Snowflake is expected to refuse to compile the SQL.
    pub compile_error: bool,
    /// The project functions the query calls.
    pub functions: Vec<CaseFunction>,
    /// The user overrides conversion uses.
    pub overrides: Vec<CaseOverride>,
    /// The Snowflake statements creating them, run before `source_sql` in a scratch schema.
    pub setup_sql: Vec<String>,
    /// Its input columns, each with its cells in row order; read by the coverage check.
    pub inputs: Vec<CaseInput>,
    /// The source SQL of each input row alone, where result rows follow input rows one for one.
    pub row_sources: Vec<String>,
}

/// One input column of a loaded case and its cells, source-dialect literals in row order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseInput {
    /// The column.
    pub column: InputColumn,
    /// Its cells.
    pub cells: Vec<String>,
}

/// What a loaded case proves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseRole {
    /// Its conversion replays on DuckDB with the recorded result.
    Conversion,
    /// Its source spelling records the same result as its canonical spelling.
    EquivalenceSource {
        /// The equivalence id.
        equivalence: String,
        /// The hash of the canonical spelling's case.
        canonical_hash: String,
    },
    /// The canonical spelling of an equivalence pair; judged with its source spelling.
    EquivalenceCanonical {
        /// The equivalence id.
        equivalence: String,
    },
    /// Its recorded column types and nullability, which analysis checks; never converted.
    Typing,
}

/// Which spelling of a case to expand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    /// The `expression` or `query` as written.
    Source,
    /// The `canonical` spelling.
    Canonical,
}

/// The hashed content of a case: everything that can change the recorded result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HashedContent<'a> {
    /// The evidence format.
    pub format: u32,
    /// The source dialect.
    pub source_dialect: &'a str,
    /// The session parameters.
    pub session_parameters: &'a BTreeMap<String, SessionValue>,
    /// The source SQL.
    pub source_sql: &'a str,
    /// `Some(false)` when the recording keeps no values; absent otherwise, so older hashes hold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<bool>,
    /// The statements creating the case's functions; absent when there are none.
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    pub setup_sql: &'a [String],
}
