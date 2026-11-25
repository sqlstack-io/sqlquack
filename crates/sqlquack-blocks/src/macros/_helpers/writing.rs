//! The writing bodies: a VARIANT as Snowflake writes it, keys in byte order, and its escapes.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::constants::{HELD_DOUBLE_FORMAT, HELD_DOUBLE_TOKEN_PATTERN, HELD_TOKEN_PATTERN};
use crate::macros::_helpers::guards::{bound, keys_unsorted, site_fidelity_error};
use crate::macros::constants::{JSON_ESCAPED_V1, TEXT_PARAMETER, VALUE_PARAMETER};
use crate::macros::main::helper_call::helper_call;
use crate::semi_structured::constants::{
    JSON_ANY_NUMBER_PATTERN, JSON_ESCAPE_HEX_LETTERS, JSON_STRING_PATTERN,
};
use crate::semi_structured::main::json_escaped::json_escaped;

/// Where the written text takes Snowflake's escapes.
#[derive(Clone, Copy)]
enum Escape {
    /// On each text that may be chosen, as the released version-1 macros do.
    EachText,
    /// Once, on the text chosen: DuckDB pastes the escape's body in at every use.
    Chosen,
}

/// The text of `json_value` with every object's keys in byte order, from DuckDB's `VARIANT`.
const RESORTED: &str = "CAST(CAST(CAST(json_value AS VARIANT) AS JSON) AS VARCHAR)";

/// A VARIANT cast to VARCHAR: a string root unquoted, JSON null as SQL NULL, others as JSON text.
pub(crate) fn text_body() -> String {
    written(&format!(
        "(CASE WHEN json_value IS NULL OR json_type(json_value) = 'NULL' THEN NULL \
         WHEN json_type(json_value) = 'VARCHAR' THEN json_extract_string(json_value, '$') \
         ELSE {} END)",
        sorted(Escape::EachText)
    ))
}

/// A VARIANT as JSON text, JSON null as `null`, as TO_JSON writes it.
pub(crate) fn json_text_body() -> String {
    written(&format!(
        "(CASE WHEN json_value IS NULL THEN NULL ELSE {} END)",
        sorted(Escape::EachText)
    ))
}

/// variant_text_v1's result, a number or boolean written as it is and other values escaped once.
pub(crate) fn text_body_v2() -> String {
    written(&format!(
        "(CASE WHEN json_value IS NULL OR json_type(json_value) = 'NULL' THEN NULL \
         WHEN json_type(json_value) = 'VARCHAR' THEN json_extract_string(json_value, '$') \
         WHEN json_type(json_value) NOT IN ('OBJECT', 'ARRAY') THEN CAST(json_value AS VARCHAR) \
         ELSE {} END)",
        sorted(Escape::Chosen)
    ))
}

/// variant_json_text_v1's result, a number, boolean or null written as it is, others escaped once.
pub(crate) fn json_text_body_v2() -> String {
    written(&format!(
        "(CASE WHEN json_value IS NULL THEN NULL \
         WHEN json_type(json_value) NOT IN ('OBJECT', 'ARRAY', 'VARCHAR') \
         THEN CAST(json_value AS VARCHAR) ELSE {} END)",
        sorted(Escape::Chosen)
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
fn sorted(escape: Escape) -> String {
    let escaped = |sql: &str| match escape {
        Escape::EachText => helper_call(JSON_ESCAPED_V1, &[sql]),
        Escape::Chosen => sql.to_owned(),
    };
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
    let chosen = bound(
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
    match escape {
        Escape::EachText => chosen,
        Escape::Chosen => bound(
            &chosen,
            "json_chosen",
            &helper_call(JSON_ESCAPED_V1, &["json_chosen"]),
        ),
    }
}

/// The numbers of the JSON text `sql`, outside strings, in byte order.
fn numbers(sql: &str) -> String {
    format!(
        "list_sort(regexp_extract_all(regexp_replace({sql}, '{}', '\"\"', 'g'), \
         '{JSON_ANY_NUMBER_PATTERN}'))",
        JSON_STRING_PATTERN.replace('\'', "''")
    )
}

/// variant_written_v1's body: a VARIANT-held value's JSON with doubles as `%.15e`, escaped.
pub(crate) fn held_written_body() -> String {
    let sql = format!("CAST({VALUE_PARAMETER} AS JSON)");
    let token = HELD_TOKEN_PATTERN.replace('\'', "''");
    let respelled = format!(
        "(CASE WHEN left(held_tokens[i], 1) = '\"' THEN held_tokens[i] \
         WHEN regexp_matches(held_tokens[i], '{HELD_DOUBLE_TOKEN_PATTERN}') \
         THEN printf('{HELD_DOUBLE_FORMAT}', CAST(held_tokens[i] AS DOUBLE)) \
         ELSE held_tokens[i] END)"
    );
    let rebuilt = bound(
        &format!("regexp_extract_all(held_text, '{token}')"),
        "held_tokens",
        &format!(
            "array_to_string(list_transform(regexp_split_to_array(held_text, '{token}'), \
             lambda held_part, i: held_part || coalesce({respelled}, '')), '')"
        ),
    );
    bound(
        &format!("CAST({sql} AS VARCHAR)"),
        "held_text",
        &format!(
            "(CASE WHEN held_text IS NULL THEN NULL ELSE {} END)",
            json_escaped(&rebuilt, &ProvenanceLog::default())
        ),
    )
}
