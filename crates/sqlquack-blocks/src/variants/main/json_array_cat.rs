//! `json_array_cat`: ARRAY_CAT, two arrays joined.

use sqlquack_diagnostics::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{
    ARRAY_CAT_LEFT_MESSAGE, ARRAY_CAT_RIGHT_MESSAGE, JSON_ARRAY_CAT,
};
use crate::variants::_helpers::objects::{bound, json_error};

/// The first array's elements then the second's; a value other than an array raises.
pub fn json_array_cat((left, right): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_CAT);
    bound(
        &format!("struct_pack(a := CAST({left} AS JSON), b := CAST({right} AS JSON))"),
        "q",
        &format!(
            "(CASE WHEN q['a'] IS NULL OR q['b'] IS NULL THEN NULL \
             WHEN json_type(q['a']) <> 'ARRAY' THEN {} \
             WHEN json_type(q['b']) <> 'ARRAY' THEN {} \
             ELSE CAST(to_json(list_concat(json_extract(q['a'], '$[*]'), \
             json_extract(q['b'], '$[*]'))) AS JSON) END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, ARRAY_CAT_LEFT_MESSAGE),
            json_error(NOT_AN_ARRAY_ARGUMENT, ARRAY_CAT_RIGHT_MESSAGE)
        ),
    )
}
