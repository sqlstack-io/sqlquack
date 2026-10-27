//! `parse_statements_with`: parses SQL as [`parse_statements`] does, treating calls as `mode` says.

use crate::expressions::tree::models::Expression;
use crate::guard::models::ParseOptions;
use crate::tokens::models::CallMode;
use crate::tokens::models::Dialect;
use crate::tokens::types::Result;

/// Parses SQL as [`parse_statements`] does, treating calls as `mode` says.
pub fn parse_statements_with(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
    mode: CallMode,
) -> Result<Vec<Expression>> {
    crate::parser::models::Dialect::get(dialect.dialect_type()).parse_with_mode(sql, options, mode)
}
