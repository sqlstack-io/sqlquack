//! `json_array_written`: ARRAY_CONSTRUCT's elements written as text, undefined elements included.

use sqlquack_core::failures::constants::VARIANT_NOT_CAST;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::bound;
use crate::arrays::constants::{JSON_ARRAY_WRITTEN, UNDEFINED_TEXT};
use crate::raises::main::raised::raised;
use crate::semi_structured::constants::ARRAY_ELEMENT_NOT_TEXT_MESSAGE;
use crate::semi_structured::main::json_serialized::json_serialized;
use crate::semi_structured::main::json_text::json_text;

/// The array as `::VARCHAR` writes it, or joined by `separator` as ARRAY_TO_STRING; SQL NULLs undefined.
pub fn json_array_written(
    values: &[String],
    separator: Option<&str>,
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_ARRAY_WRITTEN);
    let body = match separator {
        None => format!(
            "('[' || array_to_string(list_transform(a, lambda e: CASE WHEN e IS NULL THEN \
             '{UNDEFINED_TEXT}' ELSE {} END), ',') || ']')",
            json_serialized("e", log)
        ),
        Some(separator) => format!(
            "(CASE WHEN {separator} IS NULL THEN NULL \
             WHEN list_contains(list_transform(a, lambda e: json_type(e)), 'NULL') \
             THEN CAST({} AS VARCHAR) \
             ELSE array_to_string(list_transform(a, lambda e: \
             CASE WHEN e IS NULL THEN '' ELSE {} END), {separator}) END)",
            raised(&tagged_message(
                VARIANT_NOT_CAST,
                ARRAY_ELEMENT_NOT_TEXT_MESSAGE
            )),
            json_text("e", log),
        ),
    };
    let elements: Vec<String> = values
        .iter()
        .map(|value| format!("CAST({value} AS JSON)"))
        .collect();
    match values.is_empty() {
        true => bound("CAST([] AS JSON[])", "a", &body),
        false => bound(&format!("[{}]", elements.join(", ")), "a", &body),
    }
}
