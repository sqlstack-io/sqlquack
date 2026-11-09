//! `parse_source`: source SQL to statements.

use sqlquack_syntax::tokens::errors::ParseFailure;
use sqlquack_syntax::{Dialect, Expression};

use crate::parsing::_helpers::guarded::parse_guarded;

/// Parses source SQL, which may hold several statements, in its own dialect.
pub fn parse_source(sql: &str, dialect: Dialect) -> Result<Vec<Expression>, ParseFailure> {
    parse_guarded(sql, dialect)
}
