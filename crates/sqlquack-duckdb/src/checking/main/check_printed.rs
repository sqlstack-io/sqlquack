//! `check_printed`: DuckDB parses printed SQL as one statement and binds it.

use duckdb::params;

use crate::checking::_helpers::dotted_parameters::dotted_parameters;
use crate::checking::_helpers::engine::with_engine;
use crate::checking::_helpers::tables::{function_definitions, table_definitions};
use crate::checking::constants::SERIALISED_SUMMARY;
use crate::checking::errors::{CheckStage, PrintedSqlFailure};
use crate::checking::models::SerialisedSummary;
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
    with_engine(|engine| {
        let summary: SerialisedSummary = engine
            .connection
            .query_row(SERIALISED_SUMMARY, params![sql], |row| {
                Ok(SerialisedSummary {
                    error: row.get(0)?,
                    message: row.get(1)?,
                    count: row.get(2)?,
                    tables: row.get(3)?,
                    functions: row.get(4)?,
                })
            })
            .map_err(|error| failure(CheckStage::Parse, error.to_string()))?;
        if summary.error {
            return Err(failure(
                CheckStage::Parse,
                summary.message.unwrap_or_default(),
            ));
        }
        if summary.count != 1 {
            return Err(failure(
                CheckStage::StatementCount,
                format!("{} statements", summary.count),
            ));
        }
        let names: Vec<String> = serde_json::from_str(&summary.tables)
            .map_err(|error| failure(CheckStage::Parse, error.to_string()))?;
        for (name, statement) in table_definitions(&names, lookup) {
            engine.define_table(&name, &statement)?;
        }
        let called: Vec<String> = serde_json::from_str(&summary.functions)
            .map_err(|error| failure(CheckStage::Parse, error.to_string()))?;
        for (name, statement) in function_definitions(&called, functions) {
            engine.define_table(&name, &statement)?;
        }
        engine
            .connection
            .prepare(sql)
            .map(drop)
            .map_err(|error| failure(CheckStage::Bind, error.to_string()))
    })
}

fn failure(stage: CheckStage, message: String) -> PrintedSqlFailure {
    PrintedSqlFailure { stage, message }
}
