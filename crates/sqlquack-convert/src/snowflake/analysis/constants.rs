//! Pipeline constants.

/// The parsed tree's keys for a named call, scalar or aggregate.
pub const CALL_NODE_KINDS: &[&str] = &["function", "aggregate_function"];

/// The column a scalar SQL function body is converted as.
pub const SCALAR_RESULT_COLUMN: &str = "RESULT";

/// The help when strict mode refuses a known difference.
pub const STRICT_REFUSES: &str = "strict mode refuses every known difference";

/// Where help tells a user without a configuration file to allow a known difference.
pub const NEW_CONFIG_FILE: &str = "a sqlquack.toml in the project folder";
