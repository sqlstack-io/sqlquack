//! A replay in a Python process that registers the case's Python functions with SQLQuack's runtime.

use std::process::{Command, Stdio};

use duckdb::Connection;
use serde_json::json;
use sqlquack_compare::values::models::ResultSet;

use crate::replay::constants::{
    PYTHON_REPLAY_DATABASE, PYTHON_REPLAY_ERROR, PYTHON_REPLAY_REQUEST, PYTHON_REPLAY_RESULT,
    PYTHON_REPLAY_SCRIPT, PYTHON_RUNTIME_SOURCE,
};
use crate::replay::main::duckdb_version::duckdb_version;
use crate::replay::models::{PythonReplay, ReplaySettings};

/// The statement's rows as the Python process stored them, or its DuckDB error.
pub(crate) fn python_replay(
    (setup, sql): (&[String], &str),
    python: &PythonReplay,
    settings: &ReplaySettings,
) -> Result<ResultSet, String> {
    let failed = |error: std::io::Error| format!("the Python replay could not run: {error}");
    let directory = tempfile::tempdir().map_err(failed)?;
    let database = directory.path().join(PYTHON_REPLAY_DATABASE);
    let error = directory.path().join(PYTHON_REPLAY_ERROR);
    let script = directory.path().join(PYTHON_REPLAY_SCRIPT);
    let request = directory.path().join(PYTHON_REPLAY_REQUEST);
    let body = json!({
        "database": database,
        "error": error,
        "time_zone": settings.time_zone,
        "null_order": settings.null_order,
        "setup": setup,
        "functions": python.functions,
        "duckdb_version": duckdb_version().map_err(|error| error.to_string())?,
        "sql": sql,
    });
    std::fs::write(&script, PYTHON_RUNTIME_SOURCE).map_err(failed)?;
    std::fs::write(&request, body.to_string()).map_err(failed)?;
    let output = Command::new(&python.interpreter)
        .arg(&script)
        .arg(&request)
        .stdin(Stdio::null())
        .output()
        .map_err(failed)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "the Python replay failed: {}",
            stderr.lines().last().unwrap_or_default()
        ));
    }
    if let Ok(message) = std::fs::read_to_string(&error) {
        return Err(message);
    }
    let connection = Connection::open(&database).map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare(&format!("SELECT * FROM {PYTHON_REPLAY_RESULT}"))
        .map_err(|error| error.to_string())?;
    let arrow = statement
        .query_arrow([])
        .map_err(|error| error.to_string())?;
    let schema = arrow.get_schema();
    let batches: Vec<arrow::record_batch::RecordBatch> = arrow.collect();
    Ok(ResultSet { schema, batches })
}
