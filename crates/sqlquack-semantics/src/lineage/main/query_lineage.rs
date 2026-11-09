//! `query_lineage`: the column lineage of the one query in some SQL.

use sqlquack_parse::parse_statements_with;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};

use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::{LineageContext, QueryLineage};

/// The lineage of each output column of the single query `sql` holds, or why there is none.
pub fn query_lineage(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
    context: LineageContext<'_>,
) -> Result<QueryLineage, String> {
    let statements = parse_statements_with(sql, dialect, options, CallMode::FaithfulLocated)
        .map_err(|error| error.to_string())?;
    let context = LineageContext {
        sql: Some(sql),
        ..context
    };
    match statements.as_slice() {
        [statement] => Ok(statement_lineage(statement, context)),
        _ => Err(format!(
            "expected one statement, found {}",
            statements.len()
        )),
    }
}
