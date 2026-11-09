//! `json_array_compact`: ARRAY_COMPACT, an array without its null elements.

use sqlquack_core::failures::constants::NOT_AN_ARRAY_ARGUMENT;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{ARRAY_COMPACT_MESSAGE, JSON_ARRAY_COMPACT};
use crate::variants::_helpers::objects::{bound, json_error};

/// The array without JSON null elements; NULL if NULL, and a value other than an array raises.
pub fn json_array_compact(array: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_COMPACT);
    bound(
        &format!("CAST({array} AS JSON)"),
        "q",
        &format!(
            "(CASE WHEN q IS NULL THEN NULL WHEN json_type(q) <> 'ARRAY' THEN {} \
             ELSE CAST(to_json(list_filter(json_extract(q, '$[*]'), \
             lambda e: json_type(e) <> 'NULL')) AS JSON) END)",
            json_error(NOT_AN_ARRAY_ARGUMENT, ARRAY_COMPACT_MESSAGE)
        ),
    )
}
