//! Binding, raising and element keys for array building blocks.

use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::failures::models::SnowflakeCode;

use crate::raises::main::raised::raised;
use crate::semi_structured::main::json_output::json_output;
use sqlquack_core::provenance::models::ProvenanceLog;

/// `body` over `value` evaluated once, as the lambda parameter `name`.
pub(crate) fn bound(value: &str, name: &str, body: &str) -> String {
    format!("list_transform([{value}], lambda {name}: {body})[1]")
}

/// An expression raising the Snowflake error `code`, typed as JSON.
pub(crate) fn json_error(code: SnowflakeCode, message: &str) -> String {
    format!("CAST({} AS JSON)", raised(&tagged_message(code, message)))
}

/// The elements of the JSON array `sql`, as a list of JSON values.
pub(crate) fn elements(sql: &str) -> String {
    format!("json_extract({sql}, '$[*]')")
}

/// A JSON list as a JSON array.
pub(crate) fn json_array(list: &str) -> String {
    format!("CAST(to_json({list}) AS JSON)")
}

/// The text Snowflake writes for a JSON element, by which elements equal as VARIANTs compare equal.
pub(crate) fn element_key(element: &str, log: &ProvenanceLog) -> String {
    format!("CAST({} AS VARCHAR)", json_output(element, log))
}
