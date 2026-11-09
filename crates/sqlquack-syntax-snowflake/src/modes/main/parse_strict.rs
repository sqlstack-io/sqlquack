//! `parse_strict`: the statements of `sql`, refusing on any parse error or any byte no token covers.

use crate::modes::errors::StrictFailure;
use crate::parser::main::tokenize::tokenize;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::tokens::models::Dialect;

/// The statements of `sql`, refusing on any parse error or any byte no token covers.
pub fn parse_strict(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, StrictFailure> {
    let statements =
        crate::parser::main::parse_statements::parse_statements(sql, dialect, options)?;
    tokenize(sql, dialect)?;
    Ok(statements)
}
