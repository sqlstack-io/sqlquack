//! `parse_statements`: parses SQL, possibly several statements, keeping calls as written, under complexity limits.

use crate::parser::main::parse_statements_with::parse_statements_with;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::tokens::models::CallMode;
use sqlquack_syntax::tokens::models::Dialect;
use sqlquack_syntax::tokens::types::Result;

/// Parses SQL, possibly several statements, keeping calls as written, under complexity limits.
pub fn parse_statements(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>> {
    parse_statements_with(sql, dialect, options, CallMode::Faithful)
}
