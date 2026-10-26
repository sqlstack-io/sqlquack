//! `json_array_construct`: ARRAY_CONSTRUCT of JSON values.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_ARRAY_CONSTRUCT;
use crate::variants::_helpers::objects::{bound, fidelity_error, json_list};

/// A JSON array of the values; a SQL NULL element, which Snowflake keeps as undefined, raises.
pub fn json_array_construct(values: &[String], log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_CONSTRUCT);
    if values.is_empty() {
        return "CAST('[]' AS JSON)".to_owned();
    }
    bound(
        &json_list(values),
        "a",
        &format!(
            "(CASE WHEN len(list_filter(a, lambda e: e IS NULL)) > 0 THEN CAST({} AS JSON) \
             ELSE CAST(to_json(a) AS JSON) END)",
            fidelity_error("an array element Snowflake keeps as undefined")
        ),
    )
}
