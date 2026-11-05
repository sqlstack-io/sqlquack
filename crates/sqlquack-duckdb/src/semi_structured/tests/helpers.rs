use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::main::json_guarded::json_guarded;
use crate::semi_structured::main::json_serialized::json_serialized;

/// Objects nested `depth` deep, each listing key b before key a.
pub(crate) fn nested_document(depth: usize) -> String {
    (1..depth).fold(r#"{"b":1,"a":2}"#.to_owned(), |inner, _| {
        format!(r#"{{"b":{inner},"a":0}}"#)
    })
}

/// The same objects with every object's keys in byte order.
pub(crate) fn sorted_document(depth: usize) -> String {
    (1..depth).fold(r#"{"a":2,"b":1}"#.to_owned(), |inner, _| {
        format!(r#"{{"a":0,"b":{inner}}}"#)
    })
}

/// The SQL reading `document` with PARSE_JSON and writing it with TO_JSON.
pub(crate) fn written_sql(document: &str) -> String {
    let log = ProvenanceLog::default();
    let literal = format!("'{}'", document.replace('\'', "''"));
    json_serialized(&json_guarded(&literal, false, &log), &log)
}

/// What DuckDB returns for `sql`, and its error when it raises.
pub(crate) fn evaluated(sql: &str) -> (Option<String>, Option<String>) {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .query_row(&format!("SELECT {sql}"), [], |row| {
            row.get::<_, Option<String>>(0)
        })
        .map_or_else(|error| (None, Some(error.to_string())), |text| (text, None))
}
