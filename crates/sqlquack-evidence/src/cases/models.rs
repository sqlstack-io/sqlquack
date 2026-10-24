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
    /// For queries: `ordered` when the query fixes the row order, else `bag`.
    #[serde(default)]
    pub order: Option<QueryOrder>,
    /// The edge-case inputs the case covers.
    pub tags: Vec<String>,
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
    /// The category of its file.
    pub category: String,
    /// The construct of its file.
    pub construct: String,
    /// The session profile name.
    pub session_profile: String,
    /// The Snowflake session parameters it records under.
    pub session_parameters: BTreeMap<String, SessionValue>,
    /// The full source SQL that is recorded.
    pub source_sql: String,
    /// How rows compare.
    pub order: RowOrder,
    /// The content hash naming its result.
    pub hash: String,
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
}
