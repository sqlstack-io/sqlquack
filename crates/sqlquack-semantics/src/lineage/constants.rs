//! Names lineage knows: table functions' columns and generated column names.

use sqlquack_syntax::expressions::queries::models::JoinKind;

/// Built-in table functions and their columns, each derived from the function's arguments.
pub(crate) const TABLE_FUNCTION_COLUMNS: [(&str, &[&str]); 3] = [
    ("FLATTEN", &["SEQ", "KEY", "PATH", "INDEX", "VALUE", "THIS"]),
    ("SPLIT_TO_TABLE", &["SEQ", "INDEX", "VALUE"]),
    ("STRTOK_SPLIT_TO_TABLE", &["SEQ", "INDEX", "VALUE"]),
];
/// Serialized expression fields that do not change what a group key means: spans and `::` spelling.
pub(crate) const SHAPE_COSMETIC_FIELDS: [&str; 2] = ["span", "double_colon_syntax"];
/// The suffix of serialized comment fields, which group-key shapes drop.
pub(crate) const SHAPE_COMMENT_SUFFIX: &str = "comments";
/// BigQuery's array-flattening table function.
pub(crate) const UNNEST_FUNCTION: &str = "UNNEST";
/// The call name an array literal `[...]` parses to.
pub(crate) const ARRAY_FUNCTION: &str = "ARRAY";
/// The call name a `STRUCT(...)` value parses to.
pub(crate) const STRUCT_FUNCTION: &str = "STRUCT";
/// The wrapper Snowflake writes around a table function in FROM.
pub(crate) const TABLE_WRAPPER: &str = "TABLE";
/// Snowflake's name for the n-th column of a VALUES list, numbered from 1.
pub(crate) const SNOWFLAKE_VALUES_COLUMN: &str = "COLUMN";
/// Window and group aggregate functions SQLQuack's faithful parse keeps as plain calls.
pub(crate) const AGGREGATE_FUNCTIONS: &[&str] = &[
    "ANY_VALUE",
    "APPROX_COUNT_DISTINCT",
    "ARRAY_AGG",
    "ARRAY_UNION_AGG",
    "AVG",
    "BITAND_AGG",
    "BOOLAND_AGG",
    "BOOLOR_AGG",
    "COUNT",
    "COUNT_IF",
    "LISTAGG",
    "MAX",
    "MAX_BY",
    "MEDIAN",
    "MIN",
    "MIN_BY",
    "MODE",
    "OBJECT_AGG",
    "PERCENTILE_CONT",
    "STDDEV",
    "STRING_AGG",
    "SUM",
    "VARIANCE",
    "LIST",
    "APPROX_PERCENTILE",
    "APPROX_TOP_K",
    "ARG_MAX",
    "ARG_MIN",
    "ARRAY_UNIQUE_AGG",
    "BITOR_AGG",
    "BITXOR_AGG",
    "BIT_AND",
    "BIT_OR",
    "BIT_XOR",
    "BOOLXOR_AGG",
    "BOOL_AND",
    "BOOL_OR",
    "CORR",
    "COVAR_POP",
    "COVAR_SAMP",
    "HASH_AGG",
    "KURTOSIS",
    "MINHASH",
    "MINHASH_COMBINE",
    "PERCENTILE_DISC",
    "QUANTILE_CONT",
    "QUANTILE_DISC",
    "REGR_AVGX",
    "REGR_AVGY",
    "REGR_COUNT",
    "REGR_INTERCEPT",
    "REGR_R2",
    "REGR_SLOPE",
    "REGR_SXX",
    "REGR_SXY",
    "REGR_SYY",
    "SKEW",
    "STDDEV_POP",
    "STDDEV_SAMP",
    "VARIANCE_POP",
    "VARIANCE_SAMP",
    "VAR_POP",
    "VAR_SAMP",
];
/// `$n` numbers a FROM item's columns from 1.
pub(crate) const POSITIONAL_COLUMN_BASE: usize = 1;
/// Snowflake's `IDENTIFIER(...)`: a relation named at run time, not traced.
pub(crate) const IDENTIFIER_FUNCTION: &str = "IDENTIFIER";
/// Stack a recursive walk keeps free before it continues on a new segment.
pub(crate) const STACK_RED_ZONE: usize = 256 * 1024;
/// The size of each stack segment a deep walk adds.
pub(crate) const STACK_SEGMENT: usize = 4 * 1024 * 1024;
/// Iterations of a recursive CTE's lineage before it is left untraced.
pub(crate) const RECURSION_LIMIT: usize = 32;
/// Characters in an unaliased expression whose Snowflake output name lineage does not model.
pub(crate) const UNNAMED_CHARACTERS: [char; 5] = ['"', '\'', '`', '-', '/'];
/// GROUP BY and ORDER BY ordinals number outputs from 1.
pub(crate) const ORDINAL_BASE: usize = 1;
/// The type of a boolean value.
pub(crate) const BOOLEAN_TYPE: &str = "BOOLEAN";
/// The type of text: a string literal, a concatenation or a cast to unsized VARCHAR.
pub(crate) const TEXT_TYPE: &str = "VARCHAR";
/// The type of COUNT.
pub(crate) const COUNT_TYPE: &str = "BIGINT";
/// The type of a whole-number literal.
pub(crate) const INTEGER_LITERAL_TYPE: &str = "INT";
/// The type of a literal with a point or an exponent.
pub(crate) const DECIMAL_LITERAL_TYPE: &str = "DECIMAL";
/// Functions whose result has one type, whatever their arguments.
pub(crate) const FUNCTION_RESULT_TYPES: [(&str, &str); 31] = [
    ("BOOLAND_AGG", BOOLEAN_TYPE),
    ("BOOLOR_AGG", BOOLEAN_TYPE),
    ("BOOLXOR_AGG", BOOLEAN_TYPE),
    ("TIMESTAMP_NTZ_FROM_PARTS", "TIMESTAMPNTZ"),
    ("PARSE_JSON", "VARIANT"),
    ("TRY_PARSE_JSON", "VARIANT"),
    ("REGEXP_LIKE", BOOLEAN_TYPE),
    ("RLIKE", BOOLEAN_TYPE),
    ("TO_DATE", "DATE"),
    ("TRY_TO_DATE", "DATE"),
    ("TO_TIMESTAMP", "TIMESTAMP"),
    ("TO_TIMESTAMP_NTZ", "TIMESTAMP_NTZ"),
    ("TRY_TO_TIMESTAMP_NTZ", "TIMESTAMP_NTZ"),
    ("TO_VARCHAR", TEXT_TYPE),
    ("TO_CHAR", TEXT_TYPE),
    ("SUBSTRING", TEXT_TYPE),
    ("SUBSTR", TEXT_TYPE),
    ("REGEXP_SUBSTR", TEXT_TYPE),
    ("REGEXP_REPLACE", TEXT_TYPE),
    ("SPLIT_PART", TEXT_TYPE),
    ("TRIM", TEXT_TYPE),
    ("LTRIM", TEXT_TYPE),
    ("RTRIM", TEXT_TYPE),
    ("REPLACE", TEXT_TYPE),
    ("UPPER", TEXT_TYPE),
    ("LOWER", TEXT_TYPE),
    ("CONCAT", TEXT_TYPE),
    ("LISTAGG", TEXT_TYPE),
    ("TRY_TO_DOUBLE", "FLOAT"),
    ("OBJECT_AGG", "OBJECT"),
    ("ROW_NUMBER", COUNT_TYPE),
];
/// Functions whose result is the one type their value operands share: (name, first operand).
pub(crate) const OPERAND_TYPED_FUNCTIONS: [(&str, usize); 7] = [
    ("COALESCE", 0),
    ("NVL", 0),
    ("IFNULL", 0),
    ("IFF", 1),
    ("NVL2", 1),
    ("GREATEST", 0),
    ("LEAST", 0),
];
/// Functions whose result has the type of one argument: (name, its position).
pub(crate) const ARGUMENT_TYPED_FUNCTIONS: [(&str, usize); 11] = [
    ("MIN", 0),
    ("MAX", 0),
    ("ANY_VALUE", 0),
    ("NULLIF", 0),
    ("LEAD", 0),
    ("LAG", 0),
    ("FIRST_VALUE", 0),
    ("LAST_VALUE", 0),
    ("NTH_VALUE", 0),
    ("DATE_TRUNC", 1),
    ("ABS", 0),
];
/// Joins that may NULL-fill the rows of the item they join, and of the items before it.
pub(crate) const OUTER_LEFT_JOINS: [JoinKind; 9] = [
    JoinKind::Left,
    JoinKind::NaturalLeft,
    JoinKind::AsOfLeft,
    JoinKind::LeftLateral,
    JoinKind::OuterApply,
    JoinKind::LeftArray,
    JoinKind::Full,
    JoinKind::NaturalFull,
    JoinKind::Outer,
];
pub(crate) const OUTER_RIGHT_JOINS: [JoinKind; 6] = [
    JoinKind::Right,
    JoinKind::NaturalRight,
    JoinKind::AsOfRight,
    JoinKind::Full,
    JoinKind::NaturalFull,
    JoinKind::Outer,
];
/// Functions that return their first operand that is not NULL.
pub(crate) const COALESCING_FUNCTIONS: [&str; 2] = ["COALESCE", "IFNULL"];
/// Functions that count rows, never NULL.
pub(crate) const COUNTING_FUNCTIONS: [&str; 2] = ["COUNT", "COUNT_IF"];
/// Spellings of floating-point types, as `canonical_type` writes them.
pub(crate) const FLOAT_TYPES: [&str; 6] = [
    "FLOAT",
    "FLOAT4",
    "FLOAT8",
    "DOUBLE",
    "DOUBLEPRECISION",
    "REAL",
];
/// Spellings of whole-number types, as `canonical_type` writes them.
pub(crate) const INTEGER_TYPES: [&str; 6] = [
    "INT",
    "BIGINT",
    "SMALLINT",
    "TINYINT",
    "BYTEINT",
    "NUMBER(38,0)",
];
/// The arguments of an IFF-style function: the condition and its two results.
pub(crate) const CONDITIONAL_ARGUMENTS: usize = 3;
/// The star a projection may be spelled as a column name.
pub(crate) const SQL_STAR: &str = "*";
/// Functions NULL exactly when their first argument is, whatever the catalog's rules.
pub(crate) const FIRST_ARGUMENT_NULLABILITY: [&str; 2] = ["UPPER", "LOWER"];
/// The FROM items of a query with one join.
pub(crate) const SINGLE_JOIN_SOURCES: usize = 2;
/// BigQuery's analytic functions, which an OVER clause must follow.
pub(crate) const BIGQUERY_WINDOW_ONLY_FUNCTIONS: [&str; 11] = [
    "ROW_NUMBER",
    "RANK",
    "DENSE_RANK",
    "PERCENT_RANK",
    "CUME_DIST",
    "NTILE",
    "LAG",
    "LEAD",
    "FIRST_VALUE",
    "LAST_VALUE",
    "NTH_VALUE",
];
/// The field a serialized node records its source range in, which equality ignores.
pub(crate) const SPAN_FIELD: &str = "span";
/// The field a serialized node records its name in.
pub(crate) const NAME_FIELD: &str = "name";
/// Snowflake's VARIANT type.
pub(crate) const SNOWFLAKE_VARIANT_TYPE: &str = "VARIANT";
/// The aggregates of a PIVOT whose columns DuckDB names by value alone.
pub(crate) const ONE_AGGREGATE: usize = 1;
/// A timestamp written without its kind, which the profile's mapping decides.
pub(crate) const UNMAPPED_TIMESTAMP: &str = "TIMESTAMP";
/// Snowflake types whose value may be a JSON null.
pub(crate) const SNOWFLAKE_JSON_HOLDERS: [&str; 1] = ["VARIANT"];
/// Snowflake's TO_VARIANT, whose result is a JSON null only when its argument is one.
pub(crate) const SNOWFLAKE_TO_VARIANT: &str = "TO_VARIANT";
/// GROUP BY constructs whose subtotal rows NULL the keys they roll up, as functions spell them.
pub(crate) const SUBTOTAL_GROUPINGS: [&str; 3] = ["ROLLUP", "CUBE", "GROUPING SETS"];
/// Functions with a date or time part keyword argument, not a column, and its position.
pub(crate) const DATE_PART_FUNCTIONS: [(&str, usize); 9] = [
    ("DATEADD", 0),
    ("DATEDIFF", 0),
    ("TIMEADD", 0),
    ("TIMEDIFF", 0),
    ("TIMESTAMPADD", 0),
    ("TIMESTAMPDIFF", 0),
    ("DATE_PART", 0),
    ("DATE_TRUNC", 0),
    ("LAST_DAY", 1),
];
/// The code of a column no FROM item offers.
pub(crate) const UNKNOWN_COLUMN_CODE: &str = "E201";
/// The code of a FROM item with more column aliases than columns.
pub(crate) const ALIAS_COUNT_CODE: &str = "E223";
/// The code of an unqualified column several FROM items offer.
pub(crate) const AMBIGUOUS_COLUMN_CODE: &str = "E221";
/// The code of a qualifier that names no FROM item.
pub(crate) const UNKNOWN_QUALIFIER_CODE: &str = "E222";
/// Snowflake's CONNECT BY pseudo-column.
pub(crate) const CONNECT_BY_LEVEL: &str = "LEVEL";
/// The prefix of Snowflake's metadata pseudo-columns: streams, CHANGES and staged files.
pub(crate) const METADATA_PREFIX: &str = "METADATA$";
/// The prefix of a positional column reference, `$1`.
pub(crate) const POSITIONAL_PREFIX: char = '$';
/// DuckDB's COLUMNS star expression, written as a call.
pub(crate) const COLUMNS_FUNCTION: &str = "columns";
/// A DuckDB COLUMNS pattern that matches no column, worded as polyglot words it.
pub(crate) const COLUMNS_NO_MATCH: &str = "COLUMNS pattern matches no input columns";
/// Sequence pseudo-columns, read after the sequence's name.
pub(crate) const SEQUENCE_COLUMNS: [&str; 2] = ["NEXTVAL", "CURRVAL"];
/// The phase of scope-rule findings: grouping, aggregate and window placement, structure.
pub(crate) const SCOPE_PHASE: u8 = 0;
/// The phase of name findings: unknown, ambiguous and unqualifiable references.
pub(crate) const NAME_PHASE: u8 = 1;
/// The phase of type findings: comparisons, arithmetic, casts, arguments and set operations.
pub(crate) const TYPE_PHASE: u8 = 2;
/// DuckDB keywords that are not callable functions: only the bracket-free form is valid.
pub(crate) const DUCKDB_KEYWORD_ONLY_FUNCTIONS: [&str; 4] = [
    "current_time",
    "current_timestamp",
    "localtime",
    "localtimestamp",
];
/// The base of the digits in a date or time literal.
pub(crate) const DECIMAL_BASE: u32 = 10;
/// The last hour of a day in a time literal.
pub(crate) const LAST_HOUR: u32 = 23;
/// The last minute of an hour, or second of a minute, in a time literal.
pub(crate) const LAST_MINUTE_OR_SECOND: u32 = 59;
/// The warning for a value converted at run time or of a family no rule models.
pub(crate) const RUNTIME_WARNING: &str = "W216";
