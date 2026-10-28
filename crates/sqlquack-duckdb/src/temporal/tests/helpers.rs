use sqlquack_core::provenance::models::ProvenanceLog;

use crate::temporal::main::formatted_text_to_timestamp::formatted_text_to_timestamp;
use crate::temporal::main::timestamp_format::timestamp_format;

/// The block's timestamp for `text` in `format`, evaluated by DuckDB: its text, NULL, or an error.
pub(crate) fn formatted(format: &str, text: &str) -> Result<Option<String>, ()> {
    let known = timestamp_format(format).expect("the format is read");
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let sql = format!(
        "SELECT CAST({} AS VARCHAR) FROM (SELECT CAST(? AS VARCHAR) AS t)",
        formatted_text_to_timestamp("t", &known, &ProvenanceLog::default())
    );
    connection
        .query_row(&sql, duckdb::params![text], |row| row.get(0))
        .map_err(|_| ())
}
