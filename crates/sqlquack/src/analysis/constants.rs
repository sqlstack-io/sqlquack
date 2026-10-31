//! Pipeline constants.

/// The key recorded for a refusal that names no construct.
pub const STATEMENT_REFUSAL: &str = "statement";

/// The column a scalar SQL function body is converted as.
pub const SCALAR_RESULT_COLUMN: &str = "RESULT";

/// Settings every DuckDB session running converted SQL must have: the proofs hold only in UTC.
pub const REQUIRED_DUCKDB_SETTINGS: &[(&str, &str)] = &[("TimeZone", "UTC")];

/// The profile name reported when the built-in default stands in for a policy.
pub const BUILTIN_PROFILE: &str = "the built-in default";
