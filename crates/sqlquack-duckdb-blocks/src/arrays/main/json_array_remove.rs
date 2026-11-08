//! `json_array_remove`: ARRAY_REMOVE, an array without the elements equal to a value.

use sqlquack_core::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, element_key, elements, json_array, json_error};
use crate::arrays::constants::{JSON_ARRAY_REMOVE, NOT_AN_ARRAY_MESSAGE};

/// The array's elements other than those equal to the value as VARIANTs; NULL if either is NULL.
pub fn json_array_remove((array, value): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_REMOVE);
    let kept = json_array(&format!(
        "list_filter({}, lambda e: {} <> {})",
        elements("q['a']"),
        element_key("e", log),
        element_key("q['v']", log)
    ));
    bound(
        &format!("struct_pack(a := CAST({array} AS JSON), v := CAST({value} AS JSON))"),
        "q",
        &format!(
            "(CASE WHEN q['a'] IS NULL OR q['v'] IS NULL THEN NULL \
             WHEN json_type(q['a']) <> 'ARRAY' THEN {} ELSE {kept} END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE)
        ),
    )
}
