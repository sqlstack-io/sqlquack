//! Run constants.

/// The temporary table holding the query's result.
pub const ACTUAL: &str = "temp.main.sqlquack_actual";
/// The temporary table holding the expected rows, typed as the result.
pub const EXPECTED: &str = "temp.main.sqlquack_expected";
/// DuckDB's type for semi-structured values, rendered minified so equal JSON compares equal.
pub const JSON_TYPE: &str = "JSON";
/// How the result shows SQL NULL.
pub const NULL_TEXT: &str = "NULL";
