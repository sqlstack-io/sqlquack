//! Parser limits.

/// Nested function calls allowed per statement, above the parser's default, for generated SQL.
pub const MAX_FUNCTION_CALL_DEPTH: usize = 256;
