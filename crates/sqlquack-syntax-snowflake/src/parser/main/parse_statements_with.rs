//! `parse_statements_with`: parses SQL as [`parse_statements`] does, treating calls as `mode` says.

use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::tokens::models::CallMode;
use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_syntax::tokens::types::Result;

use crate::parser::_helpers::support::dialects::dialect_type;

/// Parses SQL as [`parse_statements`] does, treating calls as `mode` says.
pub fn parse_statements_with(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
    mode: CallMode,
) -> Result<Vec<Expression>> {
    crate::parser::models::Dialect::get(dialect_type(dialect)?).parse_with_mode(sql, options, mode)
}
