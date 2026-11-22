//! Helpers of the parsing tests.

use sqlquack_syntax::{CallMode, Dialect, Expression, ParseOptions};

/// The statements of `sql` with every call as written.
pub(crate) fn parse_as_written(sql: &str, dialect: Dialect) -> Option<Vec<Expression>> {
    sqlquack_syntax_snowflake::parse_statements_with(
        sql,
        dialect,
        &ParseOptions::default(),
        CallMode::AsWrittenLocated,
    )
    .ok()
}
