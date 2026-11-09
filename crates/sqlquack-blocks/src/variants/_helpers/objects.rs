//! Object entries, key lists and raising expressions for building and checking VARIANTs.

use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::failures::models::SnowflakeCode;

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::{JSON_FIDELITY, TYPED_TEXT_PATTERN};

/// `body` over `value` evaluated once, as the lambda parameter `name`.
pub(crate) fn bound(value: &str, name: &str, body: &str) -> String {
    format!("list_transform([{value}], lambda {name}: {body})[1]")
}

/// An expression raising the json_fidelity error, naming `what`.
pub(crate) fn fidelity_error(what: &str) -> String {
    raised(&format!("{JSON_FIDELITY}: {what}"))
}

/// An expression raising the Snowflake error `code`, typed as JSON.
pub(crate) fn json_error(code: SnowflakeCode, message: &str) -> String {
    format!("CAST({} AS JSON)", raised(&tagged_message(code, message)))
}

/// The entries of the JSON object `sql` as a list of `{key, value}` structs.
pub(crate) fn object_entries(sql: &str) -> String {
    format!("map_entries(CAST({sql} AS MAP(VARCHAR, JSON)))")
}

/// The JSON object of a list of `{key, value}` entries, keys in byte order as Snowflake keeps them.
pub(crate) fn sorted_object(entries: &str) -> String {
    format!("CAST(to_json(map_from_entries(list_sort({entries}))) AS JSON)")
}

/// A key list as a DuckDB list literal.
pub(crate) fn key_list(keys: &[String]) -> String {
    let quoted: Vec<String> = keys
        .iter()
        .map(|key| format!("'{}'", key.replace('\'', "''")))
        .collect();
    format!("CAST([{}] AS VARCHAR[])", quoted.join(", "))
}

/// Whether every element of the JSON array `sql` is a string; an empty array qualifies.
pub(crate) fn string_elements(sql: &str) -> String {
    format!(
        "coalesce(list_bool_and(list_transform(CAST({sql} AS JSON[]), \
         lambda e: coalesce(json_type(e) = 'VARCHAR', false))), true)"
    )
}

/// Whether the JSON value `sql` is a string in the VARIANT text of a typed value SQLQuack stores.
pub(crate) fn typed_text(sql: &str) -> String {
    format!(
        "(json_type({sql}) = 'VARCHAR' AND \
         regexp_full_match(json_extract_string({sql}, '$'), '{TYPED_TEXT_PATTERN}'))"
    )
}

/// The values cast to JSON, as a DuckDB list.
pub(crate) fn json_list(values: &[String]) -> String {
    let elements: Vec<String> = values
        .iter()
        .map(|value| format!("CAST({value} AS JSON)"))
        .collect();
    format!("[{}]", elements.join(", "))
}
