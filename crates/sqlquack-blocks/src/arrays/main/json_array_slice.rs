//! `json_array_slice`: ARRAY_SLICE, the elements from one 0-based position up to another.

use sqlquack_diagnostics::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, elements, json_array, json_error};
use crate::arrays::constants::{JSON_ARRAY_SLICE, NOT_AN_ARRAY_MESSAGE};

/// Elements `from` to before `to`, negative positions counted from the end and clamped to the array.
pub fn json_array_slice((array, from, to): (&str, &str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_SLICE);
    let clamped = |bound: &str| {
        format!(
            "(CASE WHEN q['{bound}'] < 0 THEN greatest(len(e) + q['{bound}'], 0) \
             ELSE least(q['{bound}'], len(e)) END)"
        )
    };
    let sliced = json_array(&format!(
        "list_transform([{}], lambda e: list_slice(e, {} + 1, {}))[1]",
        elements("q['a']"),
        clamped("f"),
        clamped("t")
    ));
    bound(
        &format!(
            "struct_pack(a := CAST({array} AS JSON), f := CAST({from} AS BIGINT), \
             t := CAST({to} AS BIGINT))"
        ),
        "q",
        &format!(
            "(CASE WHEN q['a'] IS NULL OR q['f'] IS NULL OR q['t'] IS NULL THEN NULL \
             WHEN json_type(q['a']) <> 'ARRAY' THEN {} ELSE {sliced} END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE)
        ),
    )
}
