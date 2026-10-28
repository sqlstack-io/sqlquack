//! `check_printed`: DuckDB parses printed SQL as one statement and binds it.

use crate::checking::_helpers::dotted_parameters::dotted_parameters;
use crate::checking::_helpers::engine::with_engine;
use crate::checking::_helpers::lexing::lexed;
use crate::checking::_helpers::tables::{function_definitions, table_definitions};
use crate::checking::constants::PARSER_ERROR;
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
    with_engine(|engine| {
        let mut definitions = table_definitions(&names, lookup);
        definitions.extend(function_definitions(&names, functions));
        engine.keep_only(&definitions.iter().map(|(name, _)| name.as_str()).collect())?;
        for (name, statement) in &definitions {
            engine.define_table(name, statement)?;
        }
        engine.connection.prepare(sql).map(drop).map_err(|error| {
            let message = error.to_string();
            let stage = match message.starts_with(PARSER_ERROR) {
                true => CheckStage::Parse,
                false => CheckStage::Bind,
            };
            failure(stage, message)
        })
    })
}

fn failure(stage: CheckStage, message: String) -> PrintedSqlFailure {
    PrintedSqlFailure { stage, message }
}
