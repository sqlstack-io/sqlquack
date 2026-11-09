//! Hash building block identifiers.

/// Signed 64-bit hashes: `duckdb.hash_value`.
pub const HASH_VALUE: &str = "duckdb.hash_value";
/// What shifts DuckDB's unsigned 64-bit hash into the signed range Snowflake's HASH returns.
pub const HASH_OFFSET: &str = "9223372036854775808";
