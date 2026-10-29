//! JSON text as Snowflake writes it: object keys in byte order at every depth, arrays in order.

use crate::semi_structured::_helpers::guards::{bound, fidelity_error, keys_unsorted};
use crate::semi_structured::constants::{
    JSON_ANY_NUMBER_PATTERN, JSON_CONTROL_ESCAPE_PREFIX, JSON_CONTROL_ESCAPE_PREFIX_PATTERN,
    JSON_ESCAPE_HEX_LETTERS, JSON_STRING_PATTERN,
};

/// The text of `json_value` with every object's keys in byte order, from DuckDB's `VARIANT`.
const RESORTED: &str = "CAST(CAST(CAST(json_value AS VARIANT) AS JSON) AS VARCHAR)";

/// What a VARIANT's root becomes when written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Root {
    /// Every value as JSON text, JSON null as `null`: TO_JSON.
    Json,
    /// A string root unquoted and JSON null as SQL NULL; other values as JSON text: `::VARCHAR`.
    Text,
}

/// The VARIANT `sql` as Snowflake writes it; unsorted objects are resorted through DuckDB's VARIANT.
pub(crate) fn sorted_text(sql: &str, root: Root) -> String {
    let resorted = bound(
        RESORTED,
        "json_resorted",
        &format!(
            "(CASE WHEN {} <> {} THEN CAST({} AS VARCHAR) ELSE {} END)",
            numbers("json_resorted"),
            numbers("json_written['text']"),
            fidelity_error("an object whose keys DuckDB could sort only by rewriting a number"),
            escaped("json_resorted"),
        ),
    );
    let checked = bound(
        &format!(
            "struct_pack(text := CAST(json_value AS VARCHAR), unsorted := {})",
            keys_unsorted("json_value")
        ),
        "json_written",
        &format!(
            "(CASE WHEN NOT json_written['unsorted'] THEN {} ELSE {resorted} END)",
            escaped("json_written['text']"),
        ),
    );
    let body = match root {
        Root::Json => format!("(CASE WHEN json_value IS NULL THEN NULL ELSE {checked} END)"),
        Root::Text => format!(
            "(CASE WHEN json_value IS NULL OR json_type(json_value) = 'NULL' THEN NULL \
             WHEN json_type(json_value) = 'VARCHAR' THEN json_extract_string(json_value, '$') \
             ELSE {checked} END)"
        ),
    };
    bound(&format!("CAST({sql} AS JSON)"), "json_value", &body)
}

/// DuckDB's JSON text `sql` with Snowflake's escapes: DEL as `\u007f`, `\u` hex in lower case.
pub(crate) fn escaped(sql: &str) -> String {
    let lowered =
        JSON_ESCAPE_HEX_LETTERS
            .chars()
            .fold("json_escaped".to_owned(), |text, letter| {
                let once = |text: String| {
                    format!(
                        "regexp_replace({text}, '{JSON_CONTROL_ESCAPE_PREFIX_PATTERN}{letter}', \
                     '{JSON_CONTROL_ESCAPE_PREFIX}{}', 'g')",
                        letter.to_ascii_lowercase()
                    )
                };
                once(once(text))
            });
    bound(
        &format!("replace({sql}, chr(127), '\\u007f')"),
        "json_escaped",
        &format!(
            "(CASE WHEN strpos(json_escaped, '\\u00') = 0 THEN json_escaped ELSE {lowered} END)"
        ),
    )
}

/// The numbers of the JSON text `sql`, outside strings, in byte order.
fn numbers(sql: &str) -> String {
    format!(
        "list_sort(regexp_extract_all(regexp_replace({sql}, '{}', '\"\"', 'g'), \
         '{JSON_ANY_NUMBER_PATTERN}'))",
        JSON_STRING_PATTERN.replace('\'', "''")
    )
}
