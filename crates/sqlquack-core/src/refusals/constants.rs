//! Constants for refusals.

/// Each Snowflake function with a bounded argument count and its most arguments, one `NAME MAX` per line, from the syntax catalogue's call grammars.
pub(crate) const SNOWFLAKE_FUNCTION_ARITIES: &str = include_str!("snowflake_function_arities.txt");
