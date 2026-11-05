//! Pipeline constants.

/// The key recorded for a refusal that names no construct.
pub const STATEMENT_REFUSAL: &str = "statement";

/// The column a scalar SQL function body is converted as.
pub const SCALAR_RESULT_COLUMN: &str = "RESULT";

/// Settings every DuckDB session running converted SQL must have: the proofs hold only in UTC.
pub const REQUIRED_DUCKDB_SETTINGS: &[(&str, &str)] = &[("TimeZone", "UTC")];

/// How a Python function's scalar values cross: VARCHAR, NUMBER, FLOAT and BOOLEAN as recorded.
pub const SCALAR_ABI: &str = "scalar-v1";
/// How a Python function's VARIANT result crosses: JSON text written by SQLQuack's runtime.
pub const VARIANT_ABI: &str = "variant-json-v1";
/// The DuckDB type a `variant-json-v1` function returns its JSON text as.
pub const VARIANT_TEXT_TYPE: &str = "VARCHAR";
/// Why a case calling Python functions refuses when no interpreter is given.
pub const PYTHON_RUNTIME_MISSING: &str =
    "a Python function, which needs a Python runtime to register it: verify with --python";

/// The header a user override's file starts with.
pub const OVERRIDE_HEADER: &str = "FUNCTION";
/// The override header property listing parameters.
pub const OVERRIDE_ARGUMENTS: &str = "arguments";
/// The override header property naming the result type.
pub const OVERRIDE_RETURNS: &str = "returns";
/// The override header property naming the function it replaces.
pub const OVERRIDE_REPLACES: &str = "replaces";
/// The override header property naming its body's dialect.
pub const OVERRIDE_DIALECT: &str = "dialect";
/// The one dialect an override body may be written in.
pub const OVERRIDE_DUCKDB: &str = "duckdb";
/// The DuckDB type a VARIANT parameter of an override is held in.
pub const VARIANT_HOLDING_TYPE: &str = "JSON";
/// The extension of a user override's file.
pub const OVERRIDE_EXTENSION: &str = "sql";
/// The parsed tree's keys for a named call, scalar or aggregate.
pub const CALL_NODE_KINDS: &[&str] = &["function", "aggregate_function"];
/// Hex digits in a SHA-256.
pub const SHA256_HEX_LENGTH: usize = 64;
/// Snowflake aggregate and window functions, which a user override may not replace.
pub const AGGREGATE_OR_WINDOW_NAMES: &[&str] = &[
    "ANY_VALUE",
    "APPROXIMATE_JACCARD_INDEX",
    "APPROXIMATE_SIMILARITY",
    "APPROX_COUNT_DISTINCT",
    "APPROX_PERCENTILE",
    "APPROX_PERCENTILE_ACCUMULATE",
    "APPROX_PERCENTILE_COMBINE",
    "APPROX_TOP_K",
    "APPROX_TOP_K_ACCUMULATE",
    "APPROX_TOP_K_COMBINE",
    "ARRAY_AGG",
    "ARRAY_UNION_AGG",
    "ARRAY_UNIQUE_AGG",
    "AVG",
    "BITAND_AGG",
    "BITOR_AGG",
    "BITXOR_AGG",
    "BIT_AND",
    "BIT_OR",
    "BIT_XOR",
    "BOOLAND_AGG",
    "BOOLOR_AGG",
    "BOOLXOR_AGG",
    "BOOL_AND",
    "BOOL_OR",
    "CONDITIONAL_CHANGE_EVENT",
    "CONDITIONAL_TRUE_EVENT",
    "CORR",
    "COUNT",
    "COUNT_IF",
    "COVAR_POP",
    "COVAR_SAMP",
    "CUME_DIST",
    "DENSE_RANK",
    "FIRST_VALUE",
    "GROUPING",
    "GROUPING_ID",
    "HASH_AGG",
    "HLL",
    "HLL_ACCUMULATE",
    "HLL_COMBINE",
    "KURTOSIS",
    "LAG",
    "LAST_VALUE",
    "LEAD",
    "LISTAGG",
    "MAX",
    "MAX_BY",
    "MEDIAN",
    "MIN",
    "MINHASH",
    "MINHASH_COMBINE",
    "MIN_BY",
    "MODE",
    "NTH_VALUE",
    "NTILE",
    "OBJECT_AGG",
    "PERCENTILE_CONT",
    "PERCENTILE_DISC",
    "PERCENT_RANK",
    "RANK",
    "RATIO_TO_REPORT",
    "REGR_AVGX",
    "REGR_AVGY",
    "REGR_COUNT",
    "REGR_INTERCEPT",
    "REGR_R2",
    "REGR_SLOPE",
    "REGR_SXX",
    "REGR_SXY",
    "REGR_SYY",
    "ROW_NUMBER",
    "SKEW",
    "STDDEV",
    "STDDEV_POP",
    "STDDEV_SAMP",
    "SUM",
    "VARIANCE",
    "VARIANCE_POP",
    "VARIANCE_SAMP",
    "VAR_POP",
    "VAR_SAMP",
];
