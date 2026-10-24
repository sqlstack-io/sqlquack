//! `duckdb_version`: the linked DuckDB's version.

use duckdb::Connection;

use crate::errors::EvidenceError;

/// The version string DuckDB reports, such as `v2.0.0`.
pub fn duckdb_version() -> Result<String, EvidenceError> {
    let connection =
        Connection::open_in_memory().map_err(|error| EvidenceError::DuckDb(error.to_string()))?;
    connection
        .query_row("SELECT version()", [], |row| row.get::<_, String>(0))
        .map_err(|error| EvidenceError::DuckDb(error.to_string()))
}
