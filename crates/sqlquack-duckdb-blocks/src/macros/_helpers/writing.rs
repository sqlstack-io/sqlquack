//! The writing bodies: a VARIANT as Snowflake writes it, keys in byte order, and its escapes.

use crate::macros::_helpers::guards::{bound, keys_unsorted, site_fidelity_error};
use crate::macros::constants::{JSON_ESCAPED_V1, TEXT_PARAMETER, VALUE_PARAMETER};
use crate::macros::main::helper_call::helper_call;
use crate::semi_structured::constants::{
    JSON_ANY_NUMBER_PATTERN, JSON_ESCAPE_HEX_LETTERS, JSON_STRING_PATTERN,
};

/// The text of `json_value` with every object's keys in byte order, from DuckDB's `VARIANT`.
const RESORTED: &str = "CAST(CAST(CAST(json_value AS VARIANT) AS JSON) AS VARCHAR)";

/// A VARIANT cast to VARCHAR: a string root unquoted, JSON null as SQL NULL, others as JSON text.
pub(crate) fn text_body() -> String {
    written(&format!(
        "(CASE WHEN json_value IS NULL OR json_type(json_value) = 'NULL' THEN NULL \
         WHEN json_type(json_value) = 'VARCHAR' THEN json_extract_string(json_value, '$') \
         ELSE {} END)",
        sorted()
    ))
}

/// A VARIANT as JSON text, JSON null as `null`, as TO_JSON writes it.
pub(crate) fn json_text_body() -> String {
    written(&format!(
        "(CASE WHEN json_value IS NULL THEN NULL ELSE {} END)",
        sorted()
    ))
}

/// The escaping body: DEL as `\u007f`, `\u` hex in lower case, escaped backslashes set aside.
pub(crate) fn escaped_body() -> String {
    let mut lowered =
        format!("replace(replace({TEXT_PARAMETER}, chr(127), '\\u007f'), '\\\\', chr(1))");
    for digit in ['0', '1'] {
        for letter in JSON_ESCAPE_HEX_LETTERS.chars() {
            lowered = format!(
                "replace({lowered}, '\\u00{digit}{letter}', '\\u00{digit}{}')",
                letter.to_ascii_lowercase()
            );
        }
    }
    format!(
        "(CASE WHEN strpos({TEXT_PARAMETER}, '\\u00') = 0 AND strpos({TEXT_PARAMETER}, chr(127)) = 0 \
         THEN {TEXT_PARAMETER} ELSE replace({lowered}, chr(1), '\\\\') END)"
    )
}

/// `body` over the VARIANT parameter as DuckDB `JSON`, bound as `json_value`.
fn written(body: &str) -> String {
    bound(
        &format!("CAST({VALUE_PARAMETER} AS JSON)"),
        "json_value",
        body,
    )
}

/// The JSON text of `json_value`, its objects' keys resorted through DuckDB's VARIANT if needed.
fn sorted() -> String {
    let escaped = |sql: &str| helper_call(JSON_ESCAPED_V1, &[sql]);
    let resorted = bound(
        RESORTED,
        "json_resorted",
        &format!(
            "(CASE WHEN {} <> {} THEN CAST({} AS VARCHAR) ELSE {} END)",
            numbers("json_resorted"),
            numbers("json_written['text']"),
            site_fidelity_error(
                "an object whose keys DuckDB could sort only by rewriting a number"
            ),
            escaped("json_resorted"),
        ),
    );
    bound(
        &format!(
            "struct_pack(text := CAST(json_value AS VARCHAR), unsorted := {})",
            keys_unsorted("json_value")
        ),
        "json_written",
        &format!(
            "(CASE WHEN NOT json_written['unsorted'] THEN {} ELSE {resorted} END)",
            escaped("json_written['text']"),
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
