//! `json_transform`: TRANSFORM, each element of an array mapped by a lambda.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::lambdas::{elements, raising_non_arrays, undefined_element};
use crate::semi_structured::constants::JSON_TRANSFORM;

/// Each element as the JSON `value` over `parameter`; NULL results and non-arrays raise.
pub fn json_transform(
    array: &str,
    (parameter, value): (&str, &str),
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_TRANSFORM);
    raising_non_arrays(
        array,
        &format!(
            "list_transform([list_transform({}, lambda {parameter}: {value})], lambda m: \
             CASE WHEN list_bool_or(list_transform(m, lambda e: e IS NULL)) THEN {} \
             ELSE CAST(to_json(m) AS JSON) END)[1]",
            elements("q"),
            undefined_element()
        ),
    )
}
