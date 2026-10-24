//! `parse_target`: DuckDB SQL to polyglot statements.

use polyglot_sql::{DialectType, Expression};

use crate::parsing::_helpers::guarded::parse_guarded;
use crate::parsing::errors::ParseFailure;

/// Parses DuckDB SQL with the same limits as source SQL.
pub fn parse_target(sql: &str) -> Result<Vec<Expression>, ParseFailure> {
    parse_guarded(sql, DialectType::DuckDB)
}
