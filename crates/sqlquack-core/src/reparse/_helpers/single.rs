//! Re-parsing printed SQL that must hold exactly one statement.

use polyglot_sql::Expression;

use crate::parsing::errors::ParseFailure;

pub(crate) fn single_statement(
    parsed: Result<Vec<Expression>, ParseFailure>,
) -> Result<Expression, String> {
    let mut statements = parsed.map_err(|failure| failure.to_string())?;
    if statements.len() != 1 {
        return Err(format!(
            "expected one statement, found {}",
            statements.len()
        ));
    }
    Ok(statements.remove(0))
}
