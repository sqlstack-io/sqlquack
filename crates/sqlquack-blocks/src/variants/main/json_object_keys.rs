//! `json_object_keys`: OBJECT_KEYS of a VARIANT.

use sqlquack_diagnostics::failures::constants::VARIANT_NOT_CAST;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_OBJECT_KEYS, NOT_AN_OBJECT_MESSAGE};
use crate::variants::_helpers::objects::{bound, json_error};

/// The object's keys as a JSON array, in its byte order; NULL or JSON null give NULL.
pub fn json_object_keys(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_OBJECT_KEYS);
    bound(
        sql,
        "o",
        &format!(
            "(CASE WHEN o IS NULL OR json_type(o) = 'NULL' THEN NULL \
             WHEN json_type(o) <> 'OBJECT' THEN {} \
             ELSE CAST(to_json(list_sort(json_keys(o))) AS JSON) END)",
            json_error(VARIANT_NOT_CAST, NOT_AN_OBJECT_MESSAGE)
        ),
    )
}
