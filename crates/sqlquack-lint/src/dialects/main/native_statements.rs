//! `native_statements`: the statements of SQL from a dialect's own strict parser.

use sqlquack_syntax::Expression;

use crate::dialects::_helpers::parse_errors::{parse_error, uncovered};
use crate::dialects::types::StrictParse;

/// The statements `parse` reads from `sql`, refusing what that engine refuses with the line and column where it stopped.
pub(crate) fn native_statements(sql: &str, parse: StrictParse) -> Result<Vec<Expression>, String> {
    parse(sql).map_err(|(position, message)| match message {
        Some(message) => parse_error(sql, position, &message),
        None => uncovered(sql, position),
    })
}
