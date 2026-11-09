//! `shared_statements`: the statements of SQL as the shared parser reads a dialect.

use sqlquack_parse::parse_statements;
use sqlquack_syntax::{Dialect, Expression, ParseOptions};

/// The statements of `sql` in `dialect` under `options`' guards, or why the shared parser refused it.
pub(crate) fn shared_statements(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, String> {
    parse_statements(sql, dialect, options).map_err(|error| error.to_string())
}
