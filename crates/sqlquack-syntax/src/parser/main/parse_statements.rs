//! `parse_statements`: parses SQL, possibly several statements, keeping calls as written, under complexity limits.

use crate::expressions::tree::models::Expression;
use crate::guard::models::ParseOptions;
use crate::parser::main::parse_statements_with::parse_statements_with;
use crate::tokens::models::CallMode;
use crate::tokens::models::Dialect;
use crate::tokens::types::Result;

/// Parses SQL, possibly several statements, keeping calls as written, under complexity limits.
pub fn parse_statements(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>> {
    parse_statements_with(sql, dialect, options, CallMode::Faithful)
}
