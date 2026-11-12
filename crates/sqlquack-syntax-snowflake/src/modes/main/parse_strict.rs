//! `parse_strict`: the statements of `sql`, refusing on any parse error or any byte no token covers.

use crate::modes::errors::StrictFailure;
use crate::parser::main::strict_statements::strict_statements;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::tokens::models::Dialect;

/// The statements of `sql`, refusing on any parse error or uncovered byte; the instruction guard's entry.
#[inline(never)]
pub fn parse_strict(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, StrictFailure> {
    strict_statements(sql, dialect, options)
}
