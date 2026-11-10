//! Pipeline constants.

/// The parsed tree's keys for a named call, scalar or aggregate.
pub const CALL_NODE_KINDS: &[&str] = &["function", "aggregate_function"];

/// The column a scalar SQL function body is converted as.
pub const SCALAR_RESULT_COLUMN: &str = "RESULT";

/// The help when the caller refuses every known difference that can return a different value.
pub const ERRORS_ONLY: &str = "you asked to refuse every known difference that can return a different \
                           value rather than raise an error, and this one can";
