//! `json_array_flatten`: ARRAY_FLATTEN, an array of arrays joined one level deep.

use sqlquack_core::failures::constants::{FLATTEN_ELEMENT_NOT_ARRAY, NOT_AN_ARRAY_ARGUMENT};
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, elements, json_array, json_error};
use crate::arrays::constants::{FLATTEN_ELEMENT_MESSAGE, JSON_ARRAY_FLATTEN, NOT_AN_ARRAY_MESSAGE};

/// The elements' elements in order; an element other than an array, JSON null among them, raises 100107.
pub fn json_array_flatten(array: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_FLATTEN);
    let flattened = json_array(&format!(
        "flatten(list_transform({}, lambda e: {}))",
        elements("q"),
        elements("e")
    ));
    bound(
        &format!("CAST({array} AS JSON)"),
        "q",
        &format!(
            "(CASE WHEN q IS NULL THEN NULL WHEN json_type(q) <> 'ARRAY' THEN {} \
             WHEN NOT coalesce(list_bool_and(list_transform({}, \
             lambda e: json_type(e) = 'ARRAY')), true) THEN {} ELSE {flattened} END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE),
            elements("q"),
            json_error(FLATTEN_ELEMENT_NOT_ARRAY, FLATTEN_ELEMENT_MESSAGE)
        ),
    )
}
