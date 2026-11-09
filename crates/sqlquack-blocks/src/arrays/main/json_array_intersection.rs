//! `json_array_intersection`: ARRAY_INTERSECTION, elements of the first array the second also holds.

use sqlquack_diagnostics::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, element_key, elements, json_array, json_error};
use crate::arrays::constants::{JSON_ARRAY_INTERSECTION, NOT_AN_ARRAY_MESSAGE};

/// The first array's elements, each kept as often as both arrays hold it, in the first's order.
pub fn json_array_intersection((left, right): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_INTERSECTION);
    let keys = |side: &str| {
        format!(
            "list_transform({}, lambda e: {})",
            elements(&format!("q['{side}']")),
            element_key("e", log)
        )
    };
    let kept = json_array(&format!(
        "list_transform([struct_pack(a := {}, b := {}, e := {})], lambda w: \
         list_filter(w['e'], lambda e, i: len(list_filter(w['a'][1:i], lambda x: x = w['a'][i])) \
         <= len(list_filter(w['b'], lambda x: x = w['a'][i]))))[1]",
        keys("a"),
        keys("b"),
        elements("q['a']")
    ));
    bound(
        &format!("struct_pack(a := CAST({left} AS JSON), b := CAST({right} AS JSON))"),
        "q",
        &format!(
            "(CASE WHEN q['a'] IS NULL OR q['b'] IS NULL THEN NULL \
             WHEN json_type(q['a']) <> 'ARRAY' OR json_type(q['b']) <> 'ARRAY' THEN {} \
             ELSE {kept} END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE)
        ),
    )
}
