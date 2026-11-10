//! `run_query`: a converted query run on DuckDB over fixtures, and its rows checked.

use crate::run::_helpers::fixtures::load_fixtures;
use crate::run::_helpers::results::{
    differences, expected_rows, ordered_query, query_result, result_table,
};
use crate::run::_helpers::session::session;
use crate::run::models::{RunInputs, RunOutcome};

/// The query's result over the fixtures, and how it differs from the expected rows, or the error.
pub fn run_query(inputs: &RunInputs<'_>) -> Result<RunOutcome, String> {
    let ordered = ordered_query(inputs.source)?;
    let connection = session(inputs.profile, inputs.macros)?;
    load_fixtures(&connection, inputs.schema, inputs.fixtures)?;
    let (columns, rows) = query_result(&connection, inputs.converted)?;
    let names: Vec<String> = columns.iter().map(|(name, _)| name.clone()).collect();
    let differences = match inputs.expect {
        Some(expect) => Some(differences(
            expected_rows(&connection, &columns, expect)?,
            rows.clone(),
            ordered,
        )),
        None => None,
    };
    Ok(RunOutcome {
        table: result_table(&names, &rows),
        rows: rows.len(),
        differences,
    })
}
