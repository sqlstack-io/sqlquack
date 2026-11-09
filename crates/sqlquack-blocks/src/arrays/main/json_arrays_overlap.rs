//! `json_arrays_overlap`: ARRAYS_OVERLAP, whether two arrays share an element.

use sqlquack_diagnostics::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, element_key, elements, json_error};
use crate::arrays::constants::{JSON_ARRAYS_OVERLAP, NOT_AN_ARRAY_MESSAGE};

/// Whether an element of one array equals one of the other's, JSON nulls equal; NULL if either is.
pub fn json_arrays_overlap((left, right): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAYS_OVERLAP);
    let keys = |side: &str| {
        format!(
            "list_transform({}, lambda e: {})",
            elements(&format!("q['{side}']")),
            element_key("e", log)
        )
    };
    bound(
        &format!("struct_pack(a := CAST({left} AS JSON), b := CAST({right} AS JSON))"),
        "q",
        &format!(
            "(CASE WHEN q['a'] IS NULL OR q['b'] IS NULL THEN NULL \
             WHEN json_type(q['a']) <> 'ARRAY' OR json_type(q['b']) <> 'ARRAY' \
             THEN CAST({} AS BOOLEAN) \
             ELSE len(list_intersect({}, {})) > 0 END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, NOT_AN_ARRAY_MESSAGE),
            keys("a"),
            keys("b")
        ),
    )
}
