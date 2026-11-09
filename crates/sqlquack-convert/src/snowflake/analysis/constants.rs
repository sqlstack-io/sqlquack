//! Pipeline constants.

/// The parsed tree's keys for a named call, scalar or aggregate.
pub const CALL_NODE_KINDS: &[&str] = &["function", "aggregate_function"];

/// The column a scalar SQL function body is converted as.
pub const SCALAR_RESULT_COLUMN: &str = "RESULT";
