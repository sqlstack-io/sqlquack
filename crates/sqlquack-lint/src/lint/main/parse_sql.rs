//! Tokenize and parse SQL the way every lint request does.

use crate::lint::models::ParsedSql;

/// Parse `sql` under `dialect` within a call-depth limit (None: the default), refusing opaque text.
pub fn parse_sql(
    sql: &str,
    dialect: &str,
    max_function_call_depth: Option<usize>,
) -> Result<ParsedSql, String> {
    let Some(parsed) = crate::dialects::main::lint_dialect::lint_dialect(dialect)? else {
        return Err(format!("SQLQuack does not parse the {dialect} dialect yet"));
    };
    crate::lint::_helpers::parsing::parse(sql, parsed, (None, max_function_call_depth))
}
