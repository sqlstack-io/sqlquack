//! `json_array_range`: ARRAY_GENERATE_RANGE, whole numbers from a start up to before an end.

use sqlquack_core::failures::constants::ZERO_RANGE_STEP;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, json_array, json_error};
use crate::arrays::constants::{JSON_ARRAY_RANGE, ZERO_STEP_MESSAGE};

/// The numbers from `start` by `step` while before `end`; a step of zero raises 100050.
pub fn json_array_range((start, end, step): (&str, &str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_RANGE);
    bound(
        &format!(
            "struct_pack(s := CAST({start} AS BIGINT), e := CAST({end} AS BIGINT), \
             p := CAST({step} AS BIGINT))"
        ),
        "q",
        &format!(
            "(CASE WHEN q['s'] IS NULL OR q['e'] IS NULL OR q['p'] IS NULL THEN NULL \
             WHEN q['p'] = 0 THEN {} ELSE {} END)",
            json_error(ZERO_RANGE_STEP, ZERO_STEP_MESSAGE),
            json_array("range(q['s'], q['e'], q['p'])")
        ),
    )
}
