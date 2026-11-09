//! `check_printed`: DuckDB parses printed SQL as one statement and binds it.

use crate::checking::_helpers::binding::bound;
use crate::checking::_helpers::dotted_parameters::dotted_parameters;
use crate::checking::_helpers::lexing::lexed;
use crate::checking::errors::{CheckStage, PrintedSqlFailure};
use crate::checking::types::{FunctionLookup, RelationLookup};

/// Succeeds when DuckDB parses `sql` as one statement and binds it over the relations and functions.
pub fn check_printed(
    sql: &str,
    (lookup, functions): (RelationLookup<'_>, FunctionLookup<'_>),
) -> Result<(), PrintedSqlFailure> {
    let dotted = dotted_parameters(sql);
    if !dotted.is_empty() {
        return Err(failure(
            CheckStage::CapturedParameter,
            format!("{} read with a dot", dotted.join(", ")),
        ));
    }
    let lexed = lexed(sql);
    if lexed.statements != 1 {
        return Err(failure(
            CheckStage::StatementCount,
            format!("{} statements", lexed.statements),
        ));
    }
    let names: Vec<String> = lexed.names.into_iter().collect();
    bound(sql, &names, (lookup, functions))
}

fn failure(stage: CheckStage, message: String) -> PrintedSqlFailure {
    PrintedSqlFailure { stage, message }
}
