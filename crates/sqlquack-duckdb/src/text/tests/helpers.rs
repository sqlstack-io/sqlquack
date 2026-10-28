use sqlquack_core::provenance::models::ProvenanceLog;

use crate::text::main::character_edit_distance::character_edit_distance;
use crate::text::main::split_to_rows::split_to_rows;

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

/// The block's (INDEX, VALUE) rows for a text split on a delimiter, evaluated by DuckDB.
pub(crate) fn split_rows(text: Option<&str>, delimiter: &str) -> Vec<(i64, String)> {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let sql = format!(
        "SELECT CAST(p.\"INDEX\" AS BIGINT), p.\"VALUE\" FROM (SELECT CAST(? AS VARCHAR) AS t) AS i \
         CROSS JOIN LATERAL ({}) AS p ORDER BY 1",
        split_to_rows("i.t", &format!("'{delimiter}'"), &ProvenanceLog::default())
    );
    let mut statement = connection.prepare(&sql).expect("the split prepares");
    statement
        .query_map(duckdb::params![text], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("the split evaluates")
        .map(|row| row.expect("a row reads"))
        .collect()
}
