use sqlquack_core::provenance::models::ProvenanceLog;

use crate::text::main::character_edit_distance::character_edit_distance;

/// The block's distance between two texts, evaluated by DuckDB.
pub(crate) fn edit_distance(left: Option<&str>, right: Option<&str>) -> Option<i64> {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let sql = format!(
        "SELECT {} FROM (SELECT CAST(? AS VARCHAR) AS l, CAST(? AS VARCHAR) AS r)",
        character_edit_distance("l", "r", &ProvenanceLog::default())
    );
    connection
        .query_row(&sql, duckdb::params![left, right], |row| row.get(0))
        .expect("the distance evaluates")
}
