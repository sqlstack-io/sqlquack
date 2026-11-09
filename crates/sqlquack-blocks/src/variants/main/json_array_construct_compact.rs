//! `json_array_construct_compact`: ARRAY_CONSTRUCT_COMPACT of JSON values.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_ARRAY_CONSTRUCT_COMPACT;
use crate::variants::_helpers::objects::{bound, fidelity_error, json_list};

/// ARRAY_CONSTRUCT_COMPACT: the non-NULL values as a JSON array; a JSON null element raises.
pub fn json_array_construct_compact(values: &[String], log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_CONSTRUCT_COMPACT);
    if values.is_empty() {
        return "CAST('[]' AS JSON)".to_owned();
    }
    bound(
        &format!(
            "list_filter({}, lambda e: e IS NOT NULL)",
            json_list(values)
        ),
        "a",
        &format!(
            "(CASE WHEN len(list_filter(a, lambda e: json_type(e) = 'NULL')) > 0 \
             THEN CAST({} AS JSON) ELSE CAST(to_json(a) AS JSON) END)",
            fidelity_error("a JSON null that ARRAY_CONSTRUCT_COMPACT may drop")
        ),
    )
}
