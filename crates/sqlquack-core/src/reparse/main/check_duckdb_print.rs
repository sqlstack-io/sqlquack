//! `check_duckdb_print`: DuckDB SQL for an intended tree, only if it parses back to that tree.

use polyglot_sql::{DialectType, Expression};

use crate::comparison::main::compare_trees::compare_trees;
use crate::parsing::main::parse_target::parse_target;
use crate::reparse::_helpers::single::single_statement;
use crate::reparse::models::PrinterCheckFailure;

/// DuckDB SQL for `intended`, returned only when it parses back to the same tree.
pub fn check_duckdb_print(intended: &Expression) -> Result<String, PrinterCheckFailure> {
    let sql = polyglot_sql::generate(intended, DialectType::DuckDB).map_err(|error| {
        PrinterCheckFailure::PrintFailed {
            message: error.to_string(),
        }
    })?;
    let reparsed = match single_statement(parse_target(&sql)) {
        Ok(reparsed) => reparsed,
        Err(message) => return Err(PrinterCheckFailure::ReparseFailed { sql, message }),
    };
    match compare_trees(intended, &reparsed) {
        None => Ok(sql),
        Some(difference) => Err(PrinterCheckFailure::Changed { sql, difference }),
    }
}
