//! Data models of expressions typed functions.

use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::JsonOptions;
use crate::expressions::operators::models::RegexOptions;
use crate::expressions::queries::models::Ordered;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::IntervalUnit;
use crate::tokens::models::Span;

/// Generic unary function (takes a single argument)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnaryFunc {
    pub this: Expression,
    /// Original function name for round-trip preservation (e.g., CHAR_LENGTH vs LENGTH)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_name: Option<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

impl UnaryFunc {
    /// Create a new UnaryFunc with an original name for round-trip preservation
    pub fn with_name(this: Expression, name: String) -> Self {
        Self {
            this,
            original_name: Some(name),
            inferred_type: None,
        }
    }
}

/// CHAR/CHR with several arguments and optional USING charset.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharFunc {
    pub args: Vec<Expression>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub charset: Option<String>,
    /// Original function name (CHAR or CHR), defaults to CHAR
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
}

/// Generic binary function (takes two arguments)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryFunc {
    pub this: Expression,
    pub expression: Expression,
    /// Original function name for round-trip preservation (e.g., NVL vs IFNULL)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_name: Option<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// Variable argument function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarArgFunc {
    pub expressions: Vec<Expression>,
    /// Original function name for round-trip preservation (e.g., COALESCE vs IFNULL)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_name: Option<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// CONCAT_WS function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConcatWs {
    pub separator: Expression,
    pub expressions: Vec<Expression>,
}

/// SUBSTRING function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubstringFunc {
    pub this: Expression,
    pub start: Expression,
    pub length: Option<Expression>,
    /// Whether SQL standard FROM/FOR syntax was used (true) vs comma-separated (false)
    #[serde(default)]
    pub from_for_syntax: bool,
}

/// OVERLAY function - OVERLAY(string PLACING replacement FROM position [FOR length])
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayFunc {
    pub this: Expression,
    pub replacement: Expression,
    pub from: Expression,
    pub length: Option<Expression>,
}

/// TRIM function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimFunc {
    pub this: Expression,
    pub characters: Option<Expression>,
    pub position: TrimPosition,
    /// Whether SQL standard syntax was used (TRIM(BOTH chars FROM str)) vs function syntax (TRIM(str))
    #[serde(default)]
    pub sql_standard_syntax: bool,
    /// Whether the position was explicitly specified (BOTH/LEADING/TRAILING) vs defaulted
    #[serde(default)]
    pub position_explicit: bool,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum TrimPosition {
    Both,
    Leading,
    Trailing,
}

/// REPLACE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplaceFunc {
    pub this: Expression,
    pub old: Expression,
    pub new: Expression,
}

/// LEFT/RIGHT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeftRightFunc {
    pub this: Expression,
    pub length: Expression,
}

/// REPEAT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepeatFunc {
    pub this: Expression,
    pub times: Expression,
}

/// LPAD/RPAD function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PadFunc {
    pub this: Expression,
    pub length: Expression,
    pub fill: Option<Expression>,
}

/// SPLIT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitFunc {
    pub this: Expression,
    pub delimiter: Expression,
}

/// REGEXP_LIKE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpFunc {
    /// Additional search boundaries and retained source semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<RegexOptions>,
    pub this: Expression,
    pub pattern: Expression,
    pub flags: Option<Expression>,
}

/// REGEXP_REPLACE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpReplaceFunc {
    /// Additional search boundaries and retained source semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<RegexOptions>,
    pub this: Expression,
    pub pattern: Expression,
    pub replacement: Expression,
    pub flags: Option<Expression>,
}

/// REGEXP_EXTRACT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpExtractFunc {
    /// Additional search boundaries and retained source semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<RegexOptions>,
    pub this: Expression,
    pub pattern: Expression,
    pub group: Option<Expression>,
}

/// ROUND function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundFunc {
    pub this: Expression,
    pub decimals: Option<Expression>,
}

/// FLOOR function with optional scale and time unit (Druid: FLOOR(time TO unit))
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloorFunc {
    pub this: Expression,
    pub scale: Option<Expression>,
    /// Time unit for Druid-style FLOOR(time TO unit) syntax
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to: Option<Expression>,
}

/// CEIL function with optional decimals and time unit (Druid: CEIL(time TO unit))
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CeilFunc {
    pub this: Expression,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decimals: Option<Expression>,
    /// Time unit for Druid-style CEIL(time TO unit) syntax
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to: Option<Expression>,
}

/// LOG function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogFunc {
    pub this: Expression,
    pub base: Option<Expression>,
}

/// CURRENT_DATE (no arguments)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentDate;

/// CURRENT_TIME
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentTime {
    pub precision: Option<u32>,
}

/// CURRENT_TIMESTAMP
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentTimestamp {
    pub precision: Option<u32>,
    /// If true, generate SYSDATE instead of CURRENT_TIMESTAMP (Oracle-specific)
    #[serde(default)]
    pub sysdate: bool,
}

/// CURRENT_TIMESTAMP_LTZ - Snowflake local timezone timestamp
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentTimestampLTZ {
    pub precision: Option<u32>,
}

/// AT TIME ZONE expression for timezone conversion
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtTimeZone {
    /// The expression to convert
    pub this: Expression,
    /// The target timezone
    pub zone: Expression,
}

/// DATE_ADD / DATE_SUB function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateAddFunc {
    pub this: Expression,
    pub interval: Expression,
    pub unit: IntervalUnit,
}

/// DATEDIFF function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateDiffFunc {
    pub this: Expression,
    pub expression: Expression,
    pub unit: Option<IntervalUnit>,
}

/// DATE_TRUNC function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateTruncFunc {
    pub this: Expression,
    pub unit: DateTimeField,
}

/// EXTRACT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFunc {
    pub this: Expression,
    pub field: DateTimeField,
}

#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateTimeField {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
    Microsecond,
    DayOfWeek,
    DayOfYear,
    Week,
    /// Week with a modifier like WEEK(monday), WEEK(sunday)
    WeekWithModifier(String),
    Quarter,
    Epoch,
    Timezone,
    TimezoneHour,
    TimezoneMinute,
    Date,
    Time,
    /// Custom datetime field for dialect-specific or arbitrary fields
    Custom(String),
}

/// TO_DATE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToDateFunc {
    pub this: Expression,
    pub format: Option<Expression>,
}

/// TO_TIMESTAMP function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToTimestampFunc {
    pub this: Expression,
    pub format: Option<Expression>,
}

/// IF function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfFunc {
    pub condition: Expression,
    pub true_value: Expression,
    pub false_value: Option<Expression>,
    /// Original function name (IF, IFF, IIF) for round-trip preservation
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_name: Option<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// NVL2 function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nvl2Func {
    pub this: Expression,
    pub true_value: Expression,
    pub false_value: Expression,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// Generic aggregate function base type
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggFunc {
    pub this: Expression,
    pub distinct: bool,
    pub filter: Option<Expression>,
    pub order_by: Vec<Ordered>,
    /// Original function name (case-preserving) when parsed from SQL
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    /// IGNORE NULLS (true) or RESPECT NULLS (false), None if not specified
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ignore_nulls: Option<bool>,
    /// BigQuery HAVING MAX/MIN inside an aggregate: the expression and whether it is MAX.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub having_max: Option<(Box<Expression>, bool)>,
    /// LIMIT inside aggregate (e.g., ARRAY_AGG(x ORDER BY y LIMIT 2))
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<Box<Expression>>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// COUNT function with optional star
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, Serialize, Deserialize)]
pub struct CountFunc {
    /// Source range of the call; metadata, not expression identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub span: Option<Span>,
    pub this: Option<Expression>,
    pub star: bool,
    pub distinct: bool,
    pub filter: Option<Expression>,
    /// IGNORE NULLS (true) or RESPECT NULLS (false)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_nulls: Option<bool>,
    /// Original function name for case preservation (e.g., "count" or "COUNT")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_name: Option<String>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

impl PartialEq for CountFunc {
    fn eq(&self, other: &Self) -> bool {
        self.this == other.this
            && self.star == other.star
            && self.distinct == other.distinct
            && self.filter == other.filter
            && self.ignore_nulls == other.ignore_nulls
            && self.original_name == other.original_name
            && self.inferred_type == other.inferred_type
    }
}

/// GROUP_CONCAT function (MySQL style)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupConcatFunc {
    pub this: Expression,
    pub separator: Option<Expression>,
    pub order_by: Option<Vec<Ordered>>,
    pub distinct: bool,
    pub filter: Option<Expression>,
    /// MySQL 8.0.19+: LIMIT n inside GROUP_CONCAT
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Box<Expression>>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// STRING_AGG function (PostgreSQL/Standard SQL)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringAggFunc {
    pub this: Expression,
    #[serde(default)]
    pub separator: Option<Expression>,
    #[serde(default)]
    pub order_by: Option<Vec<Ordered>>,
    #[serde(default)]
    pub distinct: bool,
    #[serde(default)]
    pub filter: Option<Expression>,
    /// BigQuery LIMIT inside STRING_AGG
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<Box<Expression>>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// LISTAGG function (Oracle style)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAggFunc {
    /// Retain source byte-limit and overflow semantics through AST generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    pub this: Expression,
    pub separator: Option<Expression>,
    pub on_overflow: Option<ListAggOverflow>,
    /// Vertica `USING PARAMETERS max_length = n`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<Box<Expression>>,
    pub order_by: Option<Vec<Ordered>>,
    pub distinct: bool,
    pub filter: Option<Expression>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// LISTAGG ON OVERFLOW behavior
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListAggOverflow {
    Error,
    Truncate {
        filler: Option<Box<Expression>>,
        with_count: bool,
    },
}

/// SUM_IF / COUNT_IF function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SumIfFunc {
    pub this: Expression,
    pub condition: Expression,
    pub filter: Option<Expression>,
    /// Inferred data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// APPROX_PERCENTILE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxPercentileFunc {
    pub this: Expression,
    pub percentile: Expression,
    pub accuracy: Option<Expression>,
    pub filter: Option<Expression>,
}

/// PERCENTILE_CONT / PERCENTILE_DISC function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PercentileFunc {
    pub this: Expression,
    pub percentile: Expression,
    pub order_by: Option<Vec<Ordered>>,
    pub filter: Option<Expression>,
}

/// ROW_NUMBER function (no arguments)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowNumber;

/// RANK function (DuckDB allows ORDER BY inside, Oracle allows hypothetical args with WITHIN GROUP)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rank {
    /// DuckDB: RANK(ORDER BY col) - order by inside function
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<Ordered>>,
    /// Oracle hypothetical rank: RANK(val1, val2, ...) WITHIN GROUP (ORDER BY ...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Expression>,
}

/// DENSE_RANK function (Oracle allows hypothetical args with WITHIN GROUP)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DenseRank {
    /// Oracle hypothetical rank: DENSE_RANK(val1, val2, ...) WITHIN GROUP (ORDER BY ...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Expression>,
}

/// NTILE function (DuckDB allows ORDER BY inside)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NTileFunc {
    /// num_buckets is optional to support Databricks NTILE() without arguments
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_buckets: Option<Expression>,
    /// DuckDB: NTILE(n ORDER BY col) - order by inside function
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<Ordered>>,
}

/// LEAD / LAG function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeadLagFunc {
    pub this: Expression,
    pub offset: Option<Expression>,
    pub default: Option<Expression>,
    /// None = not specified, Some(true) = IGNORE NULLS, Some(false) = RESPECT NULLS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_nulls: Option<bool>,
}

/// FIRST_VALUE / LAST_VALUE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueFunc {
    pub this: Expression,
    /// None = not specified, Some(true) = IGNORE NULLS, Some(false) = RESPECT NULLS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_nulls: Option<bool>,
    /// ORDER BY inside the function parens (e.g., DuckDB: LAST_VALUE(x ORDER BY x))
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_by: Vec<Ordered>,
}

/// NTH_VALUE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NthValueFunc {
    pub this: Expression,
    pub offset: Expression,
    /// None = not specified, Some(true) = IGNORE NULLS, Some(false) = RESPECT NULLS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_nulls: Option<bool>,
    /// Snowflake FROM FIRST (`Some(true)`) or FROM LAST (`Some(false)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_first: Option<bool>,
}

/// PERCENT_RANK function (DuckDB allows ORDER BY inside, Oracle allows hypothetical args with WITHIN GROUP)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PercentRank {
    /// DuckDB: PERCENT_RANK(ORDER BY col) - order by inside function
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<Ordered>>,
    /// Oracle hypothetical rank: PERCENT_RANK(val1, val2, ...) WITHIN GROUP (ORDER BY ...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Expression>,
}

/// CUME_DIST function (DuckDB allows ORDER BY inside, Oracle allows hypothetical args with WITHIN GROUP)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CumeDist {
    /// DuckDB: CUME_DIST(ORDER BY col) - order by inside function
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<Ordered>>,
    /// Oracle hypothetical rank: CUME_DIST(val1, val2, ...) WITHIN GROUP (ORDER BY ...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Expression>,
}

/// POSITION/INSTR function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionFunc {
    pub substring: Expression,
    pub string: Expression,
    pub start: Option<Expression>,
}

/// RANDOM function (no arguments)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Random;

/// RAND function (optional seed, or Teradata RANDOM(lower, upper))
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rand {
    pub seed: Option<Box<Expression>>,
    /// Teradata RANDOM lower bound
    #[serde(default)]
    pub lower: Option<Box<Expression>>,
    /// Teradata RANDOM upper bound
    #[serde(default)]
    pub upper: Option<Box<Expression>>,
}

/// TRUNCATE / TRUNC function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruncateFunc {
    pub this: Expression,
    pub decimals: Option<Expression>,
}

/// PI function (no arguments)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pi;

/// DECODE function (Oracle style)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodeFunc {
    pub this: Expression,
    pub search_results: Vec<(Expression, Expression)>,
    pub default: Option<Expression>,
}

/// DATE_FORMAT / FORMAT_DATE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateFormatFunc {
    pub this: Expression,
    pub format: Expression,
}

/// FROM_UNIXTIME function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromUnixtimeFunc {
    pub this: Expression,
    pub format: Option<Expression>,
}

/// UNIX_TIMESTAMP function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnixTimestampFunc {
    pub this: Option<Expression>,
    pub format: Option<Expression>,
}

/// MAKE_DATE function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MakeDateFunc {
    pub year: Expression,
    pub month: Expression,
    pub day: Expression,
}

/// MAKE_TIMESTAMP function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MakeTimestampFunc {
    pub year: Expression,
    pub month: Expression,
    pub day: Expression,
    pub hour: Expression,
    pub minute: Expression,
    pub second: Expression,
    pub timezone: Option<Expression>,
}

/// LAST_DAY function with optional date part (for BigQuery granularity like WEEK(SUNDAY))
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LastDayFunc {
    pub this: Expression,
    /// Optional date part for granularity (e.g., MONTH, YEAR, WEEK(SUNDAY))
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unit: Option<DateTimeField>,
}

/// ARRAY constructor
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayConstructor {
    pub expressions: Vec<Expression>,
    pub bracket_notation: bool,
    /// True if LIST keyword was used instead of ARRAY (DuckDB)
    pub use_list_keyword: bool,
    /// Inferred array data type from type annotation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// ARRAY_SORT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArraySortFunc {
    pub this: Expression,
    pub comparator: Option<Expression>,
    pub desc: bool,
    pub nulls_first: Option<bool>,
}

/// ARRAY_JOIN / ARRAY_TO_STRING function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayJoinFunc {
    pub this: Expression,
    pub separator: Expression,
    pub null_replacement: Option<Expression>,
}

/// UNNEST function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnnestFunc {
    pub this: Expression,
    /// Additional arguments for multi-argument UNNEST (e.g., UNNEST(arr1, arr2))
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expressions: Vec<Expression>,
    pub with_ordinality: bool,
    pub alias: Option<Identifier>,
    /// BigQuery: offset alias for WITH OFFSET AS <name>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_alias: Option<Identifier>,
    /// Inferred type of the first UNNEST output column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub inferred_type: Option<DataType>,
}

/// ARRAY_FILTER function (with lambda)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayFilterFunc {
    pub this: Expression,
    pub filter: Expression,
}

/// ARRAY_TRANSFORM / TRANSFORM function (with lambda)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayTransformFunc {
    pub this: Expression,
    pub transform: Expression,
}

/// SEQUENCE / GENERATE_SERIES function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceFunc {
    pub start: Expression,
    pub stop: Expression,
    pub step: Option<Expression>,
}

/// STRUCT constructor
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructConstructor {
    pub fields: Vec<(Option<Identifier>, Expression)>,
}

/// STRUCT_EXTRACT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructExtractFunc {
    pub this: Expression,
    pub field: Identifier,
}

/// NAMED_STRUCT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedStructFunc {
    pub pairs: Vec<(Expression, Expression)>,
}

/// MAP constructor
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapConstructor {
    pub keys: Vec<Expression>,
    pub values: Vec<Expression>,
    /// Whether curly brace syntax was used (`{'a': 1}`) vs MAP function (`MAP(...)`)
    #[serde(default)]
    pub curly_brace_syntax: bool,
    /// Whether MAP keyword was present (`MAP {'a': 1}`) vs bare curly braces (`{'a': 1}`)
    #[serde(default)]
    pub with_map_keyword: bool,
}

/// TRANSFORM_KEYS / TRANSFORM_VALUES function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformFunc {
    pub this: Expression,
    pub transform: Expression,
}

/// Exasol function call with an EMITS (col TYPE, ...) clause.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionEmits {
    /// The function call expression
    pub this: Expression,
    /// The EMITS schema definition
    pub emits: Expression,
}

/// JSON_EXTRACT / JSON_EXTRACT_SCALAR function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonExtractFunc {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,
    /// Structured SQL/JSON wrapper and error/empty behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<JsonOptions>,

    pub this: Expression,
    pub path: Expression,
    pub returning: Option<DataType>,
    /// True if parsed from -> or ->> operator syntax
    #[serde(default)]
    pub arrow_syntax: bool,
    /// True if parsed from #>> operator syntax (PostgreSQL JSONB path text extraction)
    #[serde(default)]
    pub hash_arrow_syntax: bool,
    /// Wrapper option: WITH/WITHOUT [CONDITIONAL|UNCONDITIONAL] [ARRAY] WRAPPER
    #[serde(default)]
    pub wrapper_option: Option<String>,
    /// Quotes handling: KEEP QUOTES or OMIT QUOTES
    #[serde(default)]
    pub quotes_option: Option<String>,
    /// ON SCALAR STRING flag
    #[serde(default)]
    pub on_scalar_string: bool,
    /// Error handling: NULL ON ERROR, ERROR ON ERROR, etc.
    #[serde(default)]
    pub on_error: Option<String>,
}

/// JSON path extraction
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonPathFunc {
    pub this: Expression,
    pub paths: Vec<Expression>,
}

/// JSON_OBJECT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonObjectFunc {
    pub pairs: Vec<(Expression, Expression)>,
    pub null_handling: Option<JsonNullHandling>,
    #[serde(default)]
    pub with_unique_keys: bool,
    #[serde(default)]
    pub returning_type: Option<DataType>,
    #[serde(default)]
    pub format_json: bool,
    #[serde(default)]
    pub encoding: Option<String>,
    /// For JSON_OBJECT(*) syntax
    #[serde(default)]
    pub star: bool,
}

/// JSON null handling options
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum JsonNullHandling {
    NullOnNull,
    AbsentOnNull,
}

/// JSON_SET / JSON_INSERT function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonModifyFunc {
    pub this: Expression,
    pub path_values: Vec<(Expression, Expression)>,
}

/// JSON_ARRAYAGG function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonArrayAggFunc {
    pub this: Expression,
    pub order_by: Option<Vec<Ordered>>,
    pub null_handling: Option<JsonNullHandling>,
    pub filter: Option<Expression>,
}

/// JSON_OBJECTAGG function
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonObjectAggFunc {
    pub key: Expression,
    pub value: Expression,
    pub null_handling: Option<JsonNullHandling>,
    pub filter: Option<Expression>,
}

/// CONVERT function (SQL Server style)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvertFunc {
    pub this: Expression,
    pub to: DataType,
    pub style: Option<Expression>,
}

/// Map
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Map {
    #[serde(default)]
    pub keys: Vec<Expression>,
    #[serde(default)]
    pub values: Vec<Expression>,
}

/// MapCat
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapCat {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// MapDelete
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapDelete {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// MapInsert
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapInsert {
    pub this: Box<Expression>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
    #[serde(default)]
    pub value: Option<Box<Expression>>,
    #[serde(default)]
    pub update_flag: Option<Box<Expression>>,
}

/// MapPick
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapPick {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// ScopeResolution
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeResolution {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    pub expression: Box<Expression>,
}

/// Slice
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slice {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub step: Option<Box<Expression>>,
}

/// VarMap
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarMap {
    #[serde(default)]
    pub keys: Vec<Expression>,
    #[serde(default)]
    pub values: Vec<Expression>,
}

/// MatchAgainst
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchAgainst {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub modifier: Option<Box<Expression>>,
}

/// MD5Digest
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MD5Digest {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Monthname
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monthname {
    pub this: Box<Expression>,
    #[serde(default)]
    pub abbreviated: Option<Box<Expression>>,
}

/// Ntile
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ntile {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Normalize
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Normalize {
    pub this: Box<Expression>,
    #[serde(default)]
    pub form: Option<Box<Expression>>,
    #[serde(default)]
    pub is_casefold: Option<Box<Expression>>,
}

/// Normal
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Normal {
    pub this: Box<Expression>,
    #[serde(default)]
    pub stddev: Option<Box<Expression>>,
    #[serde(default)]
    pub gen: Option<Box<Expression>>,
}

/// Predict
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Predict {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub params_struct: Option<Box<Expression>>,
}

/// MLTranslate
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLTranslate {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub params_struct: Option<Box<Expression>>,
}

/// FeaturesAtTime
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeaturesAtTime {
    pub this: Box<Expression>,
    #[serde(default)]
    pub time: Option<Box<Expression>>,
    #[serde(default)]
    pub num_rows: Option<Box<Expression>>,
    #[serde(default)]
    pub ignore_feature_nulls: Option<Box<Expression>>,
}

/// GenerateEmbedding
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateEmbedding {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub params_struct: Option<Box<Expression>>,
    #[serde(default)]
    pub is_text: Option<Box<Expression>>,
}

/// MLForecast
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLForecast {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub params_struct: Option<Box<Expression>>,
}

/// ModelAttribute
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelAttribute {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// VectorSearch
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSearch {
    pub this: Box<Expression>,
    #[serde(default)]
    pub column_to_search: Option<Box<Expression>>,
    #[serde(default)]
    pub query_table: Option<Box<Expression>>,
    #[serde(default)]
    pub query_column_to_search: Option<Box<Expression>>,
    #[serde(default)]
    pub top_k: Option<Box<Expression>>,
    #[serde(default)]
    pub distance_type: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// Quantile
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantile {
    pub this: Box<Expression>,
    #[serde(default)]
    pub quantile: Option<Box<Expression>>,
}

/// ApproxQuantile
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxQuantile {
    pub this: Box<Expression>,
    #[serde(default)]
    pub quantile: Option<Box<Expression>>,
    #[serde(default)]
    pub accuracy: Option<Box<Expression>>,
    #[serde(default)]
    pub weight: Option<Box<Expression>>,
    #[serde(default)]
    pub error_tolerance: Option<Box<Expression>>,
}

/// ApproxPercentileEstimate
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApproxPercentileEstimate {
    pub this: Box<Expression>,
    #[serde(default)]
    pub percentile: Option<Box<Expression>>,
}

/// Randn
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Randn {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Randstr
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Randstr {
    pub this: Box<Expression>,
    #[serde(default)]
    pub generator: Option<Box<Expression>>,
}

/// RangeN
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeN {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub each: Option<Box<Expression>>,
}

/// RangeBucket
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeBucket {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ReadCSV
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadCSV {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// ReadParquet
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadParquet {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Reduce
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reduce {
    pub this: Box<Expression>,
    #[serde(default)]
    pub initial: Option<Box<Expression>>,
    #[serde(default)]
    pub merge: Option<Box<Expression>>,
    #[serde(default)]
    pub finish: Option<Box<Expression>>,
}

/// RegexpExtractAll
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpExtractAll {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub group: Option<Box<Expression>>,
    #[serde(default)]
    pub parameters: Option<Box<Expression>>,
    #[serde(default)]
    pub position: Option<Box<Expression>>,
    #[serde(default)]
    pub occurrence: Option<Box<Expression>>,
}

/// RegexpILike
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpILike {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub flag: Option<Box<Expression>>,
}

/// RegexpFullMatch
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpFullMatch {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// RegexpInstr
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpInstr {
    /// Additional search boundaries and retained source semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<RegexOptions>,
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub position: Option<Box<Expression>>,
    #[serde(default)]
    pub occurrence: Option<Box<Expression>>,
    #[serde(default)]
    pub option: Option<Box<Expression>>,
    #[serde(default)]
    pub parameters: Option<Box<Expression>>,
    #[serde(default)]
    pub group: Option<Box<Expression>>,
}

/// RegexpSplit
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpSplit {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub limit: Option<Box<Expression>>,
}

/// RegexpCount
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexpCount {
    /// Additional search boundaries and retained source semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<RegexOptions>,
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub position: Option<Box<Expression>>,
    #[serde(default)]
    pub parameters: Option<Box<Expression>>,
}

/// RegrValx
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrValx {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrValy
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrValy {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrAvgy
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrAvgy {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrAvgx
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrAvgx {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrCount
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrCount {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrIntercept
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrIntercept {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrR2
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrR2 {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrSxx
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrSxx {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrSxy
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrSxy {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrSyy
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrSyy {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RegrSlope
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegrSlope {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// SafeAdd
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafeAdd {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// SafeDivide
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafeDivide {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// SafeMultiply
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafeMultiply {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// SafeSubtract
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafeSubtract {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// SHA2
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SHA2 {
    pub this: Box<Expression>,
    #[serde(default)]
    pub length: Option<i64>,
}

/// SHA2Digest
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SHA2Digest {
    pub this: Box<Expression>,
    #[serde(default)]
    pub length: Option<i64>,
}

/// SortArray
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortArray {
    pub this: Box<Expression>,
    #[serde(default)]
    pub asc: Option<Box<Expression>>,
    #[serde(default)]
    pub nulls_first: Option<Box<Expression>>,
}

/// SplitPart
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitPart {
    pub this: Box<Expression>,
    #[serde(default)]
    pub delimiter: Option<Box<Expression>>,
    #[serde(default)]
    pub part_index: Option<Box<Expression>>,
}

/// SUBSTRING_INDEX(str, delim, count)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubstringIndex {
    pub this: Box<Expression>,
    #[serde(default)]
    pub delimiter: Option<Box<Expression>>,
    #[serde(default)]
    pub count: Option<Box<Expression>>,
}

/// StandardHash
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardHash {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// StrPosition
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrPosition {
    pub this: Box<Expression>,
    #[serde(default)]
    pub substr: Option<Box<Expression>>,
    #[serde(default)]
    pub position: Option<Box<Expression>>,
    #[serde(default)]
    pub occurrence: Option<Box<Expression>>,
}

/// Search
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Search {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub json_scope: Option<Box<Expression>>,
    #[serde(default)]
    pub analyzer: Option<Box<Expression>>,
    #[serde(default)]
    pub analyzer_options: Option<Box<Expression>>,
    #[serde(default)]
    pub search_mode: Option<Box<Expression>>,
}

/// SearchIp
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchIp {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// StrToDate
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrToDate {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// StrToTime
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrToTime {
    pub this: Box<Expression>,
    pub format: String,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
    #[serde(default)]
    pub target_type: Option<Box<Expression>>,
}

/// StrToUnix
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrToUnix {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub format: Option<String>,
}

/// StrToMap
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrToMap {
    pub this: Box<Expression>,
    #[serde(default)]
    pub pair_delim: Option<Box<Expression>>,
    #[serde(default)]
    pub key_value_delim: Option<Box<Expression>>,
    #[serde(default)]
    pub duplicate_resolution_callback: Option<Box<Expression>>,
}

/// NumberToStr
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumberToStr {
    pub this: Box<Expression>,
    pub format: String,
    #[serde(default)]
    pub culture: Option<Box<Expression>>,
}

/// FromBase
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromBase {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Stuff
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stuff {
    pub this: Box<Expression>,
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub length: Option<i64>,
    pub expression: Box<Expression>,
}

/// TimeToStr
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeToStr {
    pub this: Box<Expression>,
    pub format: String,
    #[serde(default)]
    pub culture: Option<Box<Expression>>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// TimeStrToTime
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeStrToTime {
    pub this: Box<Expression>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// TsOrDsAdd
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsOrDsAdd {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub return_type: Option<Box<Expression>>,
}

/// TsOrDsDiff
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsOrDsDiff {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// TsOrDsToDate
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsOrDsToDate {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// TsOrDsToTime
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TsOrDsToTime {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub safe: Option<Box<Expression>>,
}

/// Unhex
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unhex {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// Uniform
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uniform {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub gen: Option<Box<Expression>>,
    #[serde(default)]
    pub seed: Option<Box<Expression>>,
}

/// UnixToStr
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnixToStr {
    pub this: Box<Expression>,
    #[serde(default)]
    pub format: Option<String>,
}

/// UnixToTime
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnixToTime {
    pub this: Box<Expression>,
    #[serde(default)]
    pub scale: Option<i64>,
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
    #[serde(default)]
    pub hours: Option<Box<Expression>>,
    #[serde(default)]
    pub minutes: Option<Box<Expression>>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub target_type: Option<Box<Expression>>,
}

/// Uuid
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uuid {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub is_string: Option<Box<Expression>>,
}

/// TimestampFromParts
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampFromParts {
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
    #[serde(default)]
    pub milli: Option<Box<Expression>>,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// TimestampTzFromParts
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampTzFromParts {
    #[serde(default)]
    pub zone: Option<Box<Expression>>,
}

/// Corr
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Corr {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub null_on_zero_variance: Option<Box<Expression>>,
}

/// WidthBucket
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidthBucket {
    pub this: Box<Expression>,
    #[serde(default)]
    pub min_value: Option<Box<Expression>>,
    #[serde(default)]
    pub max_value: Option<Box<Expression>>,
    #[serde(default)]
    pub num_buckets: Option<Box<Expression>>,
    #[serde(default)]
    pub threshold: Option<Box<Expression>>,
}

/// CovarSamp
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovarSamp {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// CovarPop
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovarPop {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// Week
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Week {
    pub this: Box<Expression>,
    #[serde(default)]
    pub mode: Option<Box<Expression>>,
}

/// Zipf
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zipf {
    pub this: Box<Expression>,
    #[serde(default)]
    pub elementcount: Option<Box<Expression>>,
    #[serde(default)]
    pub gen: Option<Box<Expression>>,
}
