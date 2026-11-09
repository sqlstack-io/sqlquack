//! `bare_user_is_function`: whether a bare unquoted `USER` is the niladic session-user function in this dialect.

use crate::tokens::models::DialectType;

/// Whether a bare unquoted `USER` is the niladic session-user function in this dialect.
pub fn bare_user_is_function(dialect: Option<DialectType>) -> bool {
    matches!(dialect, Some(DialectType::DuckDB))
}
