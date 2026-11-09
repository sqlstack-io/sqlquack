//! `query_lineage_and_findings`: a query's lineage and its name findings, from one parse.

use sqlquack_parse::parse_statements_with;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};

use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::NameFinding;
use crate::lineage::models::{LineageContext, QueryLineage};

/// `query_lineage` of the single query `sql` holds, with its name findings, parsed once.
pub fn query_lineage_and_findings(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
    context: LineageContext<'_>,
) -> Result<(QueryLineage, Vec<NameFinding>), String> {
    let statements = parse_statements_with(sql, dialect, options, CallMode::FaithfulLocated)
        .map_err(|error| error.to_string())?;
    let context = LineageContext {
        sql: Some(sql),
        ..context
    };
    match statements.as_slice() {
        [statement] => Ok((
            statement_lineage(statement, context),
            statement_findings(statement, context),
        )),
        _ => Err(format!(
            "expected one statement, found {}",
            statements.len()
        )),
    }
}
