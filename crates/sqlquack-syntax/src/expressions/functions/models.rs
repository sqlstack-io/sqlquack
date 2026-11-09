//! Data models of expressions functions.

use crate::tokens::models::SourceSpan;
use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::JsonOptions;
use crate::expressions::operators::models::SqlBehavior;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::Over;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;

/// A scalar function call not recognized as a dedicated `Expression` variant.
#[derive(
    sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize, Default,
)]
pub struct Function {
    /// Qualified name components preserve quoting without changing the node kind.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub qualified_name: Vec<Identifier>,
    /// Optional DEFAULT/NULL/ERROR behavior after the arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<Box<SqlBehavior>>,
    /// Dialect whose call semantics must be retained through independent generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    /// The function name, as originally written (may be schema-qualified).
    pub name: String,
    /// Positional arguments to the function.
    pub args: Vec<Expression>,
    /// Whether DISTINCT was specified inside the call (e.g. `COUNT(DISTINCT x)`).
    pub distinct: bool,
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// Whether this function uses bracket syntax (e.g., MAP[keys, values])
    #[serde(default)]
    pub use_bracket_syntax: bool,
    /// Whether this function was called without parentheses (e.g., CURRENT_TIMESTAMP vs CURRENT_TIMESTAMP())
    #[serde(default)]
    pub no_parens: bool,
    /// Whether the function name was quoted (e.g., `p.d.UdF` in BigQuery)
    #[serde(default)]
    pub quoted: bool,
    /// Source position span
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// DuckDB `EXPORT_STATE`: the call returns its aggregate state.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub export_state: bool,
}

impl Function {
    /// Construct a call with individually quoted qualified name components.
    pub fn qualified(name: Vec<Identifier>, args: Vec<Expression>) -> Self {
        Self {
            name: name
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            qualified_name: name,
            args,
            ..Default::default()
        }
    }
    pub fn new(name: impl Into<String>, args: Vec<Expression>) -> Self {
        Self {
            source_dialect: None,
            qualified_name: Vec::new(),
            on_error: None,
            name: name.into(),
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: SourceSpan::default(),
            inferred_type: None,
            export_state: false,
        }
    }
}

/// A named aggregate call with optional FILTER, ORDER BY, LIMIT and IGNORE/RESPECT NULLS.
#[derive(
    sqlquack_derive::AstNode, Debug, Clone, PartialEq, Default, Serialize, Deserialize,
)]
pub struct AggregateFunction {
    /// Dialect whose call semantics must be retained through independent generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    /// The aggregate function name (e.g. "JSON_AGG", "XMLAGG").
    pub name: String,
    /// Positional arguments.
    pub args: Vec<Expression>,
    /// Whether DISTINCT was specified.
    pub distinct: bool,
    /// Optional FILTER (WHERE ...) clause applied to the aggregate.
    pub filter: Option<Expression>,
    /// ORDER BY inside aggregate (e.g., JSON_AGG(x ORDER BY y))
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_by: Vec<Ordered>,
    /// LIMIT inside aggregate (e.g., ARRAY_CONCAT_AGG(x LIMIT 2))
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Box<Expression>>,
    /// IGNORE NULLS / RESPECT NULLS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_nulls: Option<bool>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the source span of a call parsed faithfully, when spans are recorded.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
    /// The name's parts as written, when it is qualified (as `Function` keeps them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub qualified_name: Vec<Identifier>,
    /// Whether a part of the name was quoted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quoted: bool,
    /// DuckDB `EXPORT_STATE`: the call returns its aggregate state.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub export_state: bool,
}

/// A window function call; `over` holds PARTITION BY, ORDER BY and the frame.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowFunction {
    /// The function expression (e.g. ROW_NUMBER(), SUM(amount)).
    pub this: Expression,
    /// The OVER clause defining the window partitioning, ordering, and frame.
    pub over: Over,
    /// Oracle KEEP clause: KEEP (DENSE_RANK FIRST|LAST ORDER BY ...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep: Option<Keep>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Oracle KEEP (DENSE_RANK FIRST|LAST ORDER BY ...) clause of an aggregate.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keep {
    /// true = FIRST, false = LAST
    pub first: bool,
    /// ORDER BY clause inside KEEP
    pub order_by: Vec<Ordered>,
}

/// WITHIN GROUP clause (for ordered-set aggregate functions)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithinGroup {
    /// The aggregate function (LISTAGG, PERCENTILE_CONT, etc.)
    pub this: Expression,
    /// The ORDER BY clause within the group
    pub order_by: Vec<Ordered>,
}

/// DPipe
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DPipe {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// Operator
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operator {
    pub this: Box<Expression>,
    #[serde(default)]
    pub operator: Option<Box<Expression>>,
    pub expression: Box<Expression>,
    /// Comments between OPERATOR() and the RHS expression
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
}

/// PivotAny
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PivotAny {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Aliases
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aliases {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// AtIndex
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtIndex {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// FromTimeZone
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromTimeZone {
    pub this: Box<Expression>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// Format override for a column in Teradata
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatPhrase {
    pub this: Box<Expression>,
    pub format: String,
}

/// ForIn
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForIn {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Automatically converts unit arg into a var.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeUnit {
    #[serde(default)]
    pub unit: Option<String>,
}

/// IntervalOp
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalOp {
    #[serde(default)]
    pub unit: Option<String>,
    pub expression: Box<Expression>,
}

/// HavingMax
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HavingMax {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub max: Option<Box<Expression>>,
}

/// CosineDistance
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CosineDistance {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// DotProduct
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DotProduct {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// EuclideanDistance
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EuclideanDistance {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ManhattanDistance
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManhattanDistance {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// JarowinklerSimilarity
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JarowinklerSimilarity {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Booland
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Booland {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Boolor
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Boolor {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ParameterizedAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterizedAgg {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub params: Vec<Expression>,
}

/// ArgMax
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgMax {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub count: Option<Box<Expression>>,
}

/// ArgMin
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgMin {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub count: Option<Box<Expression>>,
}

/// ApproxTopK
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxTopK {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub counters: Option<Box<Expression>>,
}

/// ApproxTopKAccumulate
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxTopKAccumulate {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// ApproxTopKCombine
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxTopKCombine {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// ApproxTopKEstimate
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxTopKEstimate {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// ApproxTopSum
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxTopSum {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub count: Option<Box<Expression>>,
}

/// ApproxQuantiles
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxQuantiles {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// Minhash
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Minhash {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// FarmFingerprint
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FarmFingerprint {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Float64
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Float64 {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// Transform
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Translate
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Translate {
    pub this: Box<Expression>,
    #[serde(default)]
    pub from_: Option<Box<Expression>>,
    #[serde(default)]
    pub to: Option<Box<Expression>>,
}

/// Grouping
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grouping {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// GroupingId
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupingId {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Anonymous
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anonymous {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// AnonymousAggFunc
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnonymousAggFunc {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// CombinedAggFunc
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombinedAggFunc {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// CombinedParameterizedAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombinedParameterizedAgg {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub params: Vec<Expression>,
}

/// HashAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HashAgg {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Hll
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hll {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Apply
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Apply {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ToBoolean
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToBoolean {
    pub this: Box<Expression>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// List
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Materialize MAP constructor over a subquery or `key => value` entries.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToMap {
    /// Either a Select subquery or a Struct containing PropertyEQ entries
    pub this: Box<Expression>,
}

/// Pad
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub fill_pattern: Option<Box<Expression>>,
    #[serde(default)]
    pub is_left: Option<Box<Expression>>,
}

/// ToChar
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToChar {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub nlsparam: Option<Box<Expression>>,
    #[serde(default)]
    pub is_numeric: Option<Box<Expression>>,
}

/// StringFunc - String type conversion function (BigQuery STRING)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringFunc {
    pub this: Box<Expression>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// ToNumber
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToNumber {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<Box<Expression>>,
    #[serde(default)]
    pub nlsparam: Option<Box<Expression>>,
    #[serde(default)]
    pub precision: Option<Box<Expression>>,
    #[serde(default)]
    pub scale: Option<Box<Expression>>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
    #[serde(default)]
    pub safe_name: Option<Box<Expression>>,
}

/// ToDouble
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToDouble {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// ToDecfloat
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToDecfloat {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
}

/// TryToDecfloat
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TryToDecfloat {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
}

/// ToFile
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToFile {
    pub this: Box<Expression>,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// Columns
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Columns {
    pub this: Box<Expression>,
    #[serde(default)]
    pub unpack: Option<Box<Expression>>,
}

/// ConvertToCharset
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvertToCharset {
    pub this: Box<Expression>,
    #[serde(default)]
    pub dest: Option<Box<Expression>>,
    #[serde(default)]
    pub source: Option<Box<Expression>>,
}

/// ConvertTimezone
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvertTimezone {
    #[serde(default)]
    pub source_tz: Option<Box<Expression>>,
    #[serde(default)]
    pub target_tz: Option<Box<Expression>>,
    #[serde(default)]
    pub timestamp: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// GenerateSeries
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateSeries {
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub end: Option<Box<Expression>>,
    #[serde(default)]
    pub step: Option<Box<Expression>>,
    #[serde(default)]
    pub is_end_exclusive: Option<Box<Expression>>,
}

/// AIAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AIAgg {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// AIClassify
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AIClassify {
    pub this: Box<Expression>,
    #[serde(default)]
    pub categories: Option<Box<Expression>>,
    #[serde(default)]
    pub config: Option<Box<Expression>>,
}

/// ArrayAll
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayAll {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ArrayAny
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayAny {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ArrayConstructCompact
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayConstructCompact {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// StPoint
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StPoint {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub null: Option<Box<Expression>>,
}

/// StDistance
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StDistance {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub use_spheroid: Option<Box<Expression>>,
}

/// StringToArray
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringToArray {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub null: Option<Box<Expression>>,
}

/// ArraySum
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArraySum {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// ObjectAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectAgg {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// CastToStrType
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastToStrType {
    pub this: Box<Expression>,
    #[serde(default)]
    pub to: Option<Box<Expression>>,
}

/// CheckJson
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckJson {
    pub this: Box<Expression>,
}

/// CheckXml
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckXml {
    pub this: Box<Expression>,
    #[serde(default)]
    pub disable_auto_convert: Option<Box<Expression>>,
}

/// TranslateCharacters
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslateCharacters {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub with_error: Option<Box<Expression>>,
}

/// CurrentSchemas
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentSchemas {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// CurrentDatetime
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentDatetime {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Localtime
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Localtime {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Localtimestamp
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Localtimestamp {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Systimestamp
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Systimestamp {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// CurrentSchema
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentSchema {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// CurrentUser
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// SessionUser - MySQL/PostgreSQL SESSION_USER function
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionUser;

/// JSONPathRoot - Represents $ in JSON path expressions
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathRoot;

/// UtcTime
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtcTime {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// UtcTimestamp
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtcTimestamp {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// TimestampFunc - TIMESTAMP constructor function
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampFunc {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
    #[serde(default)]
    pub with_tz: Option<bool>,
    #[serde(default)]
    pub safe: Option<bool>,
}

/// DateBin
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateBin {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
    #[serde(default)]
    pub origin: Option<Box<Expression>>,
}

/// Datetime
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Datetime {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// DatetimeAdd
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatetimeAdd {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// DatetimeSub
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatetimeSub {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// DatetimeDiff
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatetimeDiff {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// DatetimeTrunc
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatetimeTrunc {
    pub this: Box<Expression>,
    pub unit: String,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// Dayname
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dayname {
    pub this: Box<Expression>,
    #[serde(default)]
    pub abbreviated: Option<Box<Expression>>,
}

/// MakeInterval
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MakeInterval {
    #[serde(default)]
    pub year: Option<Box<Expression>>,
    #[serde(default)]
    pub month: Option<Box<Expression>>,
    #[serde(default)]
    pub week: Option<Box<Expression>>,
    #[serde(default)]
    pub day: Option<Box<Expression>>,
    #[serde(default)]
    pub hour: Option<Box<Expression>>,
    #[serde(default)]
    pub minute: Option<Box<Expression>>,
    #[serde(default)]
    pub second: Option<Box<Expression>>,
}

/// PreviousDay
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviousDay {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Elt
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Elt {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// TimestampAdd
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampAdd {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TimestampSub
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampSub {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TimestampDiff
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampDiff {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TimeSlice
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSlice {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    pub unit: String,
    #[serde(default)]
    pub kind: Option<String>,
}

/// TimeAdd
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeAdd {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TimeSub
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSub {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TimeDiff
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeDiff {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TimeTrunc
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeTrunc {
    pub this: Box<Expression>,
    pub unit: String,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// DateFromParts
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateFromParts {
    #[serde(default)]
    pub year: Option<Box<Expression>>,
    #[serde(default)]
    pub month: Option<Box<Expression>>,
    #[serde(default)]
    pub day: Option<Box<Expression>>,
    #[serde(default)]
    pub allow_overflow: Option<Box<Expression>>,
}

/// TimeFromParts
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeFromParts {
    #[serde(default)]
    pub hour: Option<Box<Expression>>,
    #[serde(default)]
    pub min: Option<Box<Expression>>,
    #[serde(default)]
    pub sec: Option<Box<Expression>>,
    #[serde(default)]
    pub nano: Option<Box<Expression>>,
    #[serde(default)]
    pub fractions: Option<Box<Expression>>,
    #[serde(default)]
    pub precision: Option<i64>,
}

/// DecodeCase
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodeCase {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Decrypt
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decrypt {
    pub this: Box<Expression>,
    #[serde(default)]
    pub passphrase: Option<Box<Expression>>,
    #[serde(default)]
    pub aad: Option<Box<Expression>>,
    #[serde(default)]
    pub encryption_method: Option<Box<Expression>>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// DecryptRaw
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecryptRaw {
    pub this: Box<Expression>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
    #[serde(default)]
    pub iv: Option<Box<Expression>>,
    #[serde(default)]
    pub aad: Option<Box<Expression>>,
    #[serde(default)]
    pub encryption_method: Option<Box<Expression>>,
    #[serde(default)]
    pub aead: Option<Box<Expression>>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// Encode
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Encode {
    pub this: Box<Expression>,
    #[serde(default)]
    pub charset: Option<Box<Expression>>,
}

/// Encrypt
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Encrypt {
    pub this: Box<Expression>,
    #[serde(default)]
    pub passphrase: Option<Box<Expression>>,
    #[serde(default)]
    pub aad: Option<Box<Expression>>,
    #[serde(default)]
    pub encryption_method: Option<Box<Expression>>,
}

/// EncryptRaw
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptRaw {
    pub this: Box<Expression>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
    #[serde(default)]
    pub iv: Option<Box<Expression>>,
    #[serde(default)]
    pub aad: Option<Box<Expression>>,
    #[serde(default)]
    pub encryption_method: Option<Box<Expression>>,
}

/// EqualNull
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqualNull {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ToBinary
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToBinary {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// Base64DecodeBinary
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base64DecodeBinary {
    pub this: Box<Expression>,
    #[serde(default)]
    pub alphabet: Option<Box<Expression>>,
}

/// Base64DecodeString
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base64DecodeString {
    pub this: Box<Expression>,
    #[serde(default)]
    pub alphabet: Option<Box<Expression>>,
}

/// Base64Encode
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base64Encode {
    pub this: Box<Expression>,
    #[serde(default)]
    pub max_line_length: Option<Box<Expression>>,
    #[serde(default)]
    pub alphabet: Option<Box<Expression>>,
}

/// TryBase64DecodeBinary
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TryBase64DecodeBinary {
    pub this: Box<Expression>,
    #[serde(default)]
    pub alphabet: Option<Box<Expression>>,
}

/// TryBase64DecodeString
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TryBase64DecodeString {
    pub this: Box<Expression>,
    #[serde(default)]
    pub alphabet: Option<Box<Expression>>,
}

/// GapFill
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GapFill {
    pub this: Box<Expression>,
    #[serde(default)]
    pub ts_column: Option<Box<Expression>>,
    #[serde(default)]
    pub bucket_width: Option<Box<Expression>>,
    #[serde(default)]
    pub partitioning_columns: Option<Box<Expression>>,
    #[serde(default)]
    pub value_columns: Option<Box<Expression>>,
    #[serde(default)]
    pub origin: Option<Box<Expression>>,
    #[serde(default)]
    pub ignore_nulls: Option<Box<Expression>>,
}

/// GenerateDateArray
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateDateArray {
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub end: Option<Box<Expression>>,
    #[serde(default)]
    pub step: Option<Box<Expression>>,
}

/// GenerateTimestampArray
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateTimestampArray {
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub end: Option<Box<Expression>>,
    #[serde(default)]
    pub step: Option<Box<Expression>>,
}

/// GetExtract
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetExtract {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Getbit
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Getbit {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub zero_is_msb: Option<Box<Expression>>,
}

/// OverflowTruncateBehavior
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverflowTruncateBehavior {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub with_count: Option<Box<Expression>>,
}

/// HexEncode
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HexEncode {
    pub this: Box<Expression>,
    #[serde(default)]
    pub case: Option<Box<Expression>>,
}

/// Compress
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compress {
    pub this: Box<Expression>,
    #[serde(default)]
    pub method: Option<String>,
}

/// DecompressBinary
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompressBinary {
    pub this: Box<Expression>,
    pub method: String,
}

/// DecompressString
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompressString {
    pub this: Box<Expression>,
    pub method: String,
}

/// Xor
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Xor {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Nullif
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nullif {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// JSON
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSON {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub with_: Option<Box<Expression>>,
    #[serde(default)]
    pub unique: bool,
}

/// JSONPath
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPath {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub escape: Option<Box<Expression>>,
}

/// JSONPathFilter
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathFilter {
    pub this: Box<Expression>,
}

/// JSONPathKey
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathKey {
    pub this: Box<Expression>,
}

/// JSONPathRecursive
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathRecursive {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// JSONPathScript
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathScript {
    pub this: Box<Expression>,
}

/// JSONPathSlice
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathSlice {
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub end: Option<Box<Expression>>,
    #[serde(default)]
    pub step: Option<Box<Expression>>,
}

/// JSONPathSelector
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathSelector {
    pub this: Box<Expression>,
}

/// JSONPathSubscript
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathSubscript {
    pub this: Box<Expression>,
}

/// JSONPathUnion
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONPathUnion {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Format
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Format {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONKeys
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONKeys {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONKeyValue
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONKeyValue {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// JSONKeysAtDepth
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONKeysAtDepth {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub mode: Option<Box<Expression>>,
}

/// JSONObject
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONObject {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub null_handling: Option<Box<Expression>>,
    #[serde(default)]
    pub unique_keys: Option<Box<Expression>>,
    #[serde(default)]
    pub return_type: Option<Box<Expression>>,
    #[serde(default)]
    pub encoding: Option<Box<Expression>>,
}

/// JSONObjectAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONObjectAgg {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub null_handling: Option<Box<Expression>>,
    #[serde(default)]
    pub unique_keys: Option<Box<Expression>>,
    #[serde(default)]
    pub return_type: Option<Box<Expression>>,
    #[serde(default)]
    pub encoding: Option<Box<Expression>>,
}

/// JSONBObjectAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONBObjectAgg {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// JSONArray
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONArray {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub null_handling: Option<Box<Expression>>,
    #[serde(default)]
    pub return_type: Option<Box<Expression>>,
    #[serde(default)]
    pub strict: Option<Box<Expression>>,
}

/// JSONArrayAgg
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONArrayAgg {
    pub this: Box<Expression>,
    #[serde(default)]
    pub order: Option<Box<Expression>>,
    #[serde(default)]
    pub null_handling: Option<Box<Expression>>,
    #[serde(default)]
    pub return_type: Option<Box<Expression>>,
    #[serde(default)]
    pub strict: Option<Box<Expression>>,
}

/// JSONExists
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONExists {
    pub this: Box<Expression>,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub passing: Option<Box<Expression>>,
    #[serde(default)]
    pub on_condition: Option<Box<Expression>>,
    #[serde(default)]
    pub from_dcolonqmark: Option<Box<Expression>>,
}

/// JSONColumnDef
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONColumnDef {
    /// Typed column definition; legacy dialects may use the textual kind field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<DataType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    /// Structured SQL/JSON wrapper and error/empty behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<JsonOptions>,

    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub format_json: bool,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub nested_schema: Option<Box<Expression>>,
    #[serde(default)]
    pub ordinality: Option<Box<Expression>>,
}

/// JSONSchema
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONSchema {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONSet
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONSet {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONStripNulls
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONStripNulls {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub include_arrays: Option<Box<Expression>>,
    #[serde(default)]
    pub remove_empty: Option<Box<Expression>>,
}

/// JSONValue
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    /// Structured SQL/JSON wrapper and error/empty behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<JsonOptions>,

    pub this: Box<Expression>,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub returning: Option<Box<Expression>>,
    #[serde(default)]
    pub on_condition: Option<Box<Expression>>,
}

/// JSONValueArray
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONValueArray {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// JSONRemove
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONRemove {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONTable
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONTable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    /// Structured SQL/JSON wrapper and error/empty behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<JsonOptions>,

    pub this: Box<Expression>,
    #[serde(default)]
    pub schema: Option<Box<Expression>>,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub error_handling: Option<Box<Expression>>,
    #[serde(default)]
    pub empty_handling: Option<Box<Expression>>,
}

/// JSONType
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONType {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// ObjectInsert
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectInsert {
    pub this: Box<Expression>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
    #[serde(default)]
    pub value: Option<Box<Expression>>,
    #[serde(default)]
    pub update_flag: Option<Box<Expression>>,
}

/// OpenJSONColumnDef
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenJSONColumnDef {
    pub this: Box<Expression>,
    pub kind: String,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub as_json: Option<Box<Expression>>,
    /// The parsed data type for proper generation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<DataType>,
}

/// OpenJSON
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenJSON {
    pub this: Box<Expression>,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONBExists
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONBExists {
    pub this: Box<Expression>,
    #[serde(default)]
    pub path: Option<Box<Expression>>,
}

/// JSONCast
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONCast {
    pub this: Box<Expression>,
    pub to: DataType,
}

/// JSONExtract
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONExtract {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub only_json_types: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub variant_extract: Option<Box<Expression>>,
    #[serde(default)]
    pub json_query: Option<Box<Expression>>,
    #[serde(default)]
    pub option: Option<Box<Expression>>,
    #[serde(default)]
    pub quote: Option<Box<Expression>>,
    #[serde(default)]
    pub on_condition: Option<Box<Expression>>,
    #[serde(default)]
    pub requires_json: Option<Box<Expression>>,
}

/// JSONExtractQuote
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONExtractQuote {
    #[serde(default)]
    pub option: Option<Box<Expression>>,
    #[serde(default)]
    pub scalar: Option<Box<Expression>>,
}

/// JSONExtractArray
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONExtractArray {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// JSONExtractScalar
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONExtractScalar {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub only_json_types: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub json_type: Option<Box<Expression>>,
    #[serde(default)]
    pub scalar_only: Option<Box<Expression>>,
}

/// JSONBExtractScalar
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONBExtractScalar {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub json_type: Option<Box<Expression>>,
}

/// JSONFormat
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONFormat {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
    #[serde(default)]
    pub is_json: Option<Box<Expression>>,
    #[serde(default)]
    pub to_json: Option<Box<Expression>>,
}

/// JSONArrayAppend
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONArrayAppend {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// JSONArrayContains
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONArrayContains {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub json_type: Option<Box<Expression>>,
}

/// JSONArrayInsert
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSONArrayInsert {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// ParseJSON
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseJSON {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// ParseUrl
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseUrl {
    pub this: Box<Expression>,
    #[serde(default)]
    pub part_to_extract: Option<Box<Expression>>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
    #[serde(default)]
    pub permissive: Option<Box<Expression>>,
}

/// ParseIp
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseIp {
    pub this: Box<Expression>,
    #[serde(default)]
    pub type_: Option<Box<Expression>>,
    #[serde(default)]
    pub permissive: Option<Box<Expression>>,
}

/// ParseTime
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseTime {
    pub this: Box<Expression>,
    pub format: String,
}

/// ParseDatetime
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParseDatetime {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// XMLElement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XMLElement {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub evalname: Option<Box<Expression>>,
}

/// XMLGet
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XMLGet {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub instance: Option<Box<Expression>>,
}

/// XMLTable
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XMLTable {
    pub this: Box<Expression>,
    #[serde(default)]
    pub namespaces: Option<Box<Expression>>,
    #[serde(default)]
    pub passing: Option<Box<Expression>>,
    #[serde(default)]
    pub columns: Vec<Expression>,
    #[serde(default)]
    pub by_ref: Option<Box<Expression>>,
}

/// XMLKeyValueOption
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XMLKeyValueOption {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}
