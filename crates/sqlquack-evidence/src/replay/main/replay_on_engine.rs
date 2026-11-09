//! `replay_on_engine`: run SQL in a fresh DuckDB, through Python when it calls Python functions.

use duckdb::Connection;
use sqlquack_compare::values::models::ResultSet;

use crate::replay::_helpers::python_replay::python_replay;
use crate::replay::models::{PythonReplay, ReplaySettings};

/// `setup` then `sql` in a new single-threaded in-memory database with `settings`, or the error.
pub fn replay_on_engine(
    (setup, sql): (&[String], &str),
    python: Option<&PythonReplay>,
    settings: &ReplaySettings,
) -> Result<ResultSet, String> {
    if let Some(python) = python.filter(|python| !python.functions.is_empty()) {
        return python_replay((setup, sql), python, settings);
    }
    let connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    connection
        .execute_batch(&format!(
            "SET TimeZone = '{}'; SET default_null_order = '{}'; SET threads = 1;",
            settings.time_zone.replace('\'', "''"),
            settings.null_order.replace('\'', "''")
        ))
        .map_err(|error| error.to_string())?;
    for statement in setup {
        connection
            .execute_batch(statement)
            .map_err(|error| error.to_string())?;
    }
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let arrow = statement
        .query_arrow([])
        .map_err(|error| error.to_string())?;
    let schema = arrow.get_schema();
    let batches: Vec<arrow::record_batch::RecordBatch> = arrow.collect();
    Ok(ResultSet { schema, batches })
}
