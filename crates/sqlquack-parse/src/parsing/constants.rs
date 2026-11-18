//! Parser limits.

/// Nested function calls allowed per statement, above the parser's default, for generated SQL.
pub const MAX_FUNCTION_CALL_DEPTH: usize = 256;

/// The highest function-call limit a caller may set, the fixed nesting limit every parse keeps.
pub const MAX_CALL_DEPTH_CEILING: usize = 512;

/// The guard code of a parse refused for function calls nested past the limit.
pub const FUNCTION_NESTING_CODE: &str = "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED";

/// The guard code of a parse refused for brackets nested past the fixed ceiling.
pub const NESTING_CODE: &str = "E_GUARD_NESTING_DEPTH_EXCEEDED";
