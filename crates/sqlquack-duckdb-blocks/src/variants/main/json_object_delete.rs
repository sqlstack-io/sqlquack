//! `json_object_delete`: OBJECT_DELETE of literal keys.

use sqlquack_core::failures::constants::VARIANT_NOT_CAST;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_OBJECT_DELETE, NOT_AN_OBJECT_MESSAGE};
use crate::variants::_helpers::objects::{
    bound, json_error, key_list, object_entries, sorted_object,
};

/// The object without `keys`, matched exactly; NULL or JSON null give NULL, a non-object raises.
pub fn json_object_delete(sql: &str, keys: &[String], log: &ProvenanceLog) -> String {
    log.building_block(JSON_OBJECT_DELETE);
    bound(
        sql,
        "o",
        &format!(
            "(CASE WHEN o IS NULL OR json_type(o) = 'NULL' THEN NULL WHEN json_type(o) <> 'OBJECT' THEN {} ELSE {} END)",
            json_error(VARIANT_NOT_CAST, NOT_AN_OBJECT_MESSAGE),
            sorted_object(&format!(
                "list_filter({}, lambda e: NOT list_contains({}, e['key']))",
                object_entries("o"),
                key_list(keys)
            ))
        ),
    )
}
