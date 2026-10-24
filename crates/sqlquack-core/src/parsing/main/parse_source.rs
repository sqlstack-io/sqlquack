//! `parse_source`: source SQL to polyglot statements.

use polyglot_sql::Expression;

use crate::parsing::_helpers::guarded::parse_guarded;
use crate::parsing::errors::ParseFailure;
use crate::parsing::models::SourceDialect;

/// Parses source SQL, which may hold several statements, in its own dialect.
pub fn parse_source(sql: &str, dialect: SourceDialect) -> Result<Vec<Expression>, ParseFailure> {
    parse_guarded(sql, dialect.polyglot())
}
