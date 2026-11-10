//! The policy schema. Every struct rejects unknown keys.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::policy::_helpers::parsing::parse_policy;
use crate::policy::errors::PolicyError;

/// The whole policy file.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
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
    /// Limits on converting one statement; absent, the built-in defaults apply.
    #[serde(default)]
    pub conversion_budget: Option<ConversionBudgetPolicy>,
}

/// How long converting one statement may take and how much SQL it may emit, before it is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversionBudgetPolicy {
    /// Wall time for converting one statement, in milliseconds.
    pub milliseconds: u64,
    /// Emitted SQL may be this many times the source text the statement came from.
    pub size_ratio: u64,
    /// Emitted SQL up to this many bytes is always within budget.
    pub minimum_bytes: u64,
}

/// Comparator settings.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnNameMatching {
    /// Names must be byte-identical.
    #[default]
    Exact,
    /// Names must be equal after ASCII upper-casing; Snowflake upper-cases unquoted names.
    CaseInsensitive,
}

/// DuckDB's `default_null_order` values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullOrder {
    /// NULLs first in both directions.
    NullsFirst,
    /// NULLs last in both directions.
    NullsLast,
    /// NULLs first ascending, last descending.
    NullsFirstOnAscLastOnDesc,
    /// NULLs last ascending, first descending.
    #[default]
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampTzComparison {
    /// The same instant.
    Instant,
    /// The same instant and the same zone label.
    #[default]
    InstantAndZone,
}

/// Allowed float distance in units in the last place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
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
    /// Parsing and formatting parameters set while recording and checked, but not hashed.
    #[serde(default)]
    pub pinned: BTreeMap<String, SessionValue>,
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

impl SessionValue {
    /// The value as text: booleans in lower case, integers in digits, text as written.
    pub fn text(&self) -> String {
        match self {
            Self::Boolean(flag) => flag.to_string(),
            Self::Integer(number) => number.to_string(),
            Self::Text(text) => text.clone(),
        }
    }
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
    /// How the recorded cases are replayed: converting with it, or refusing it.
    pub replay: DivergenceMode,
    /// What users get without a `sqlquack.toml` saying otherwise.
    pub default: BuiltinDefault,
    /// Why values differ and what stays consistent.
    pub reason: String,
    /// Whether relying on it can return a different value, or only raise.
    pub failure: DivergenceFailure,
    /// DuckDB's own errors that are this divergence's failure, as `*` patterns over a first line.
    #[serde(default)]
    pub native_errors: Vec<String>,
    /// A named assumption: relying on it gives a wrong value, not an error, where it does not hold.
    #[serde(default)]
    pub assumption: bool,
}

/// What relying on a known divergence can do where DuckDB and Snowflake differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceFailure {
    /// The converted SQL raises wherever DuckDB could differ, and never returns a different value.
    Loud,
    /// The converted SQL can return a different value.
    Value,
}

/// How the recorded cases are replayed with a known divergence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceMode {
    /// Convert with a warning naming the location and the reason.
    Warn,
    /// Refuse.
    Refuse,
}

/// The built-in default for a known divergence, which a user's `[allow]` can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinDefault {
    /// Converted with a warning unless the user refuses it.
    Allow,
    /// Refused unless the user allows it.
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

/// A policy file's text, validated as `load_policy` validates a file; for callers without files.
impl FromStr for Policy {
    type Err = PolicyError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_policy(text)
    }
}
