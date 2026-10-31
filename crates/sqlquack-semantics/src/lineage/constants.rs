//! Names lineage knows: table functions' columns and generated column names.

/// Built-in table functions and their columns, each derived from the function's arguments.
pub(crate) const TABLE_FUNCTION_COLUMNS: [(&str, &[&str]); 3] = [
    ("FLATTEN", &["SEQ", "KEY", "PATH", "INDEX", "VALUE", "THIS"]),
    ("SPLIT_TO_TABLE", &["SEQ", "INDEX", "VALUE"]),
    ("STRTOK_SPLIT_TO_TABLE", &["SEQ", "INDEX", "VALUE"]),
];
/// The wrapper Snowflake writes around a table function in FROM.
pub(crate) const TABLE_WRAPPER: &str = "TABLE";
/// Snowflake's name for the n-th column of a VALUES list, numbered from 1.
pub(crate) const SNOWFLAKE_VALUES_COLUMN: &str = "COLUMN";
/// DuckDB's name for the n-th column of a VALUES list, numbered from 1.
pub(crate) const DUCKDB_VALUES_COLUMN: &str = "col";
/// The DuckDB profile's name, which numbers VALUES columns its own way.
pub(crate) const DUCKDB_PROFILE: &str = "DuckDB";
/// Window and group aggregate functions SQLQuack's faithful parse keeps as plain calls.
pub(crate) const AGGREGATE_FUNCTIONS: [&str; 24] = [
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
];
/// `$n` numbers a FROM item's columns from 1.
pub(crate) const POSITIONAL_COLUMN_BASE: usize = 1;
/// Snowflake's `IDENTIFIER(...)`: a relation named at run time, not traced.
pub(crate) const IDENTIFIER_FUNCTION: &str = "IDENTIFIER";
/// Iterations of a recursive CTE's lineage before it is left untraced.
pub(crate) const RECURSION_LIMIT: usize = 32;
/// Characters in an unaliased expression whose Snowflake output name lineage does not model.
pub(crate) const UNNAMED_CHARACTERS: [char; 5] = ['"', '\'', '`', '-', '/'];
