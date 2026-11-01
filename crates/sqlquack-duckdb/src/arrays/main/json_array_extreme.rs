//! `json_array_extreme`: ARRAY_MAX and ARRAY_MIN, the last or first element in ARRAY_SORT's order.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, elements};
use crate::arrays::constants::JSON_ARRAY_EXTREME;
use crate::arrays::main::json_array_sort::json_array_sort;
use crate::arrays::main::json_to_array::json_to_array;

/// The greatest or least element, JSON nulls greatest, NULL if empty; a non-array as a one-element one.
pub fn json_array_extreme(array: &str, greatest: bool, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_EXTREME);
    let position = if greatest { "-1" } else { "1" };
    bound(
        &json_array_sort(&json_to_array(array, log), log),
        "s",
        &format!(
            "(CASE WHEN s IS NULL OR json_array_length(s) = 0 THEN NULL \
             ELSE CAST({}[{position}] AS JSON) END)",
            elements("s")
        ),
    )
}
