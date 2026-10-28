//! The holding types of the check's tables, and the text it recognises.

/// The DuckDB type semi-structured Snowflake values are held in (decision 3: JSON, not VARIANT).
pub const SEMI_STRUCTURED_HOLDING_TYPE: &str = "JSON";

/// The keyword opening a DuckDB lambda in emitted SQL.
pub const LAMBDA_KEYWORD: &str = "lambda ";
/// How DuckDB's message for SQL it cannot parse starts.
pub const PARSER_ERROR: &str = "Parser Error";
/// How the statement defining a function stub starts.
pub const MACRO_PREFIX: &str = "CREATE OR REPLACE MACRO";
