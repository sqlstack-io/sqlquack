//! Parser limits.

/// Nested function calls allowed per statement, above the parser's default, for generated SQL.
pub const MAX_FUNCTION_CALL_DEPTH: usize = 256;

/// The maximum is 512. The statement and every expression around the calls also count toward that limit, so at 512 a bare SELECT can nest at most 511 calls, and fewer fit inside an alias, comparison or CTE.
pub const MAX_CALL_DEPTH_CEILING: usize = 512;

/// What a refusal at the ceiling adds: the statement's tree, not only its calls, meets the limit.
pub const CALL_DEPTH_CEILING_NOTE: &str = "The maximum is 512. The statement and every expression around the calls also count toward that limit, so at 512 a bare SELECT can nest at most 511 calls, and fewer fit inside an alias, comparison or CTE.";

/// The guard code of a parse refused for function calls nested past the limit.
pub const FUNCTION_NESTING_CODE: &str = "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED";

/// The guard code of a parse refused for brackets nested past the fixed ceiling.
pub const NESTING_CODE: &str = "E_GUARD_NESTING_DEPTH_EXCEEDED";

/// The guard code of a parse refused for a statement tree nested past the fixed ceiling.
pub const AST_DEPTH_CODE: &str = "E_GUARD_AST_DEPTH_EXCEEDED";
