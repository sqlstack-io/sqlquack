//! `query_name_findings`: the name findings of the one query in some SQL.

use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::models::LineageContext;
use crate::lineage::models::NameFinding;

/// The name findings of the single query `sql` holds, or why it cannot be read.
pub fn query_name_findings(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
    context: LineageContext<'_>,
) -> Result<Vec<NameFinding>, String> {
    let statements = parse_statements_with(sql, dialect, options, CallMode::FaithfulLocated)
        .map_err(|error| error.to_string())?;
    let context = LineageContext {
        sql: Some(sql),
        ..context
    };
    match statements.as_slice() {
        [statement] => Ok(statement_findings(statement, context)),
        _ => Err(format!(
            "expected one statement, found {}",
            statements.len()
        )),
    }
}
