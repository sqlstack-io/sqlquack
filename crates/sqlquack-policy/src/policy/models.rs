//! The policy schema. Every struct rejects unknown keys.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The whole policy file.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// The schema version; see `SCHEMA_VERSION`.
    pub schema_version: u32,
    /// How recorded and replayed results are compared.
    pub comparator: ComparatorPolicy,
    /// Session settings cases are recorded and replayed under, by profile name.
    pub session_profiles: BTreeMap<String, SessionProfile>,
    /// The edge-case input tags every case file of a category must cover, by category.
    pub edge_cases: BTreeMap<String, EdgeCaseRequirement>,
    /// Constructs whose source values DuckDB cannot reproduce but whose behaviour is consistent.
    #[serde(default)]
    pub divergences: BTreeMap<String, Divergence>,
}

/// Comparator settings.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparatorPolicy {
    /// How result column names are matched.
    pub column_names: ColumnNameMatching,
    /// DuckDB's `default_null_order` during replay.
    pub null_order: NullOrder,
    /// How zoned timestamps compare.
    pub timestamp_tz: TimestampTzComparison,
    /// Allowed distance in units in the last place, by replayed float type.
    pub float_ulps: FloatUlps,
    /// Snowflake and DuckDB result types that count as equal.
    pub type_equality: Vec<TypeEquality>,
}

/// How result column names are matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnNameMatching {
    /// Names must be byte-identical.
    Exact,
    /// Names must be equal after ASCII upper-casing; Snowflake upper-cases unquoted names.
    CaseInsensitive,
}

/// DuckDB's `default_null_order` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullOrder {
    /// NULLs first in both directions.
    NullsFirst,
    /// NULLs last in both directions.
    NullsLast,
    /// NULLs first ascending, last descending.
    NullsFirstOnAscLastOnDesc,
    /// NULLs last ascending, first descending.
    NullsLastOnAscFirstOnDesc,
}

impl NullOrder {
    /// The DuckDB setting value.
    pub fn duckdb_setting(self) -> &'static str {
        match self {
            Self::NullsFirst => "nulls_first",
            Self::NullsLast => "nulls_last",
            Self::NullsFirstOnAscLastOnDesc => "nulls_first_on_asc_last_on_desc",
            Self::NullsLastOnAscFirstOnDesc => "nulls_last_on_asc_first_on_desc",
        }
    }
}

/// How zoned timestamps compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampTzComparison {
    /// The same instant.
    Instant,
    /// The same instant and the same zone label.
    InstantAndZone,
}

/// Allowed float distance in units in the last place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloatUlps {
    /// For 32-bit replayed floats.
    pub float32: u32,
    /// For 64-bit replayed floats.
    pub float64: u32,
}

/// A Snowflake result type and the DuckDB result type that counts as equal to it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeEquality {
    /// A Snowflake type pattern such as `NUMBER(p,s)`; lower-case names bind parameters.
    pub snowflake: String,
    /// A DuckDB type pattern such as `DECIMAL(p,s)`, using only names the Snowflake side binds.
    pub duckdb: String,
    /// Why the two types hold the same values.
    pub reason: String,
}

/// Session settings for recording and replay.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionProfile {
    /// What the profile is for.
    pub description: String,
    /// Snowflake session parameters set while recording; part of every case hash.
    pub snowflake: BTreeMap<String, SessionValue>,
    /// DuckDB's `TimeZone` during replay.
    pub duckdb_time_zone: String,
}

/// A session parameter value.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SessionValue {
    /// A boolean parameter.
    Boolean(bool),
    /// An integer parameter.
    Integer(i64),
    /// A text parameter.
    Text(String),
}

/// The input tags a category's case files must cover between them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeCaseRequirement {
    /// What the category covers.
    pub description: String,
    /// Tags that at least one case of every case file must carry.
    pub required: Vec<String>,
}

/// A known divergence and how each mode treats it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    /// Construct keys such as `fn:HASH`.
    pub constructs: Vec<String>,
    /// Treatment in the default mode.
    pub default: DivergenceMode,
    /// Treatment in strict mode.
    pub strict: DivergenceMode,
    /// Why values differ and what stays consistent.
    pub reason: String,
}

/// How a mode treats a known divergence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceMode {
    /// Convert with a warning naming the location and the reason.
    Warn,
    /// Refuse.
    Refuse,
}

/// A parsed type pattern such as `NUMBER(p,s)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypePattern {
    /// The upper-case type name.
    pub base: String,
    /// The parameters.
    pub arguments: Vec<PatternArgument>,
}

/// One type-pattern parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternArgument {
    /// A fixed number.
    Literal(i64),
    /// A lower-case name bound by the first match.
    Variable(String),
}
