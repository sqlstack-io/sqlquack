//! Databricks's own strict parser, with where it stopped when it refuses SQL.

use sqlquack_parse::databricks::{StrictFailure, parse_strict};
use sqlquack_syntax::Expression;

/// The statements Databricks's own parser reads from `sql`; on refusal, its byte position and parse message, if any.
pub(crate) fn strict_parse(sql: &str) -> Result<Vec<Expression>, (usize, Option<String>)> {
    match parse_strict(sql) {
        Ok(parsed) => Ok(parsed.statements),
        Err(StrictFailure::Parse(error)) => Err((error.position, Some(error.message))),
        Err(StrictFailure::Uncovered { position }) => Err((position, None)),
    }
}
