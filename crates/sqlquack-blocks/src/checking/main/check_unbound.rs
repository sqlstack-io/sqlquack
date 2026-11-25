//! `check_unbound`: printed SQL is one statement with no lambda parameter read with a dot.

use crate::checking::_helpers::dotted_parameters::dotted_parameters;
use crate::checking::_helpers::lexing::lexed;
use crate::checking::errors::{CheckStage, PrintedSqlFailure};

/// The names `sql` reads, when it is one statement reading no lambda parameter with a dot.
pub fn check_unbound(sql: &str) -> Result<Vec<String>, PrintedSqlFailure> {
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
    Ok(lexed.names.into_iter().collect())
}

fn failure(stage: CheckStage, message: String) -> PrintedSqlFailure {
    PrintedSqlFailure { stage, message }
}
