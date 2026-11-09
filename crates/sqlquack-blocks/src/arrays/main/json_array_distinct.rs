//! `json_array_distinct`: ARRAY_DISTINCT, each element once, at its first place.

use sqlquack_diagnostics::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, element_key, elements, json_array, json_error};
use crate::arrays::constants::{JSON_ARRAY_DISTINCT, NOT_AN_ARRAY_MESSAGE};

/// The array's elements without repeats, elements equal as VARIANTs counting once.
pub fn json_array_distinct(array: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_DISTINCT);
    let kept = json_array(&format!(
        "list_transform([struct_pack(k := list_transform({}, lambda e: {}), e := {})], lambda w: \
         list_filter(w['e'], lambda e, i: NOT list_contains(w['k'][1:i - 1], w['k'][i])))[1]",
        elements("q"),
        element_key("e", log),
        elements("q")
    ));
    bound(
        &format!("CAST({array} AS JSON)"),
        "q",
        &format!(
            "(CASE WHEN q IS NULL THEN NULL WHEN json_type(q) <> 'ARRAY' THEN {} ELSE {kept} END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE)
        ),
    )
}
