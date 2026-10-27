//! `parse_strict`: the statements of `sql`, refusing on any parse error or any byte no token covers.

use crate::expressions::tree::models::Expression;
use crate::guard::models::ParseOptions;
use crate::lossless::main::tokenize_lossless::tokenize_lossless;
use crate::modes::errors::StrictFailure;
use crate::tokens::models::Dialect;

/// The statements of `sql`, refusing on any parse error or any byte no token covers.
pub fn parse_strict(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, StrictFailure> {
    let statements =
        crate::parser::main::parse_statements::parse_statements(sql, dialect, options)?;
    tokenize_lossless(sql, dialect)?;
    Ok(statements)
}
