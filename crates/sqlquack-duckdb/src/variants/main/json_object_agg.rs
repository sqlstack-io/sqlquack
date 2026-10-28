//! `json_object_agg`: OBJECT_AGG, an object of each row's key and VARIANT value.

use sqlquack_core::failures::constants::OBJECT_KEY_EXISTS;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_OBJECT_AGG, KEY_EXISTS_MESSAGE};
use crate::variants::_helpers::objects::{bound, fidelity_error, json_error, sorted_object};

/// The rows' keys and values as one object; repeated keys and JSON null values raise.
pub fn json_object_agg((key, value): (&str, &str), over: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_OBJECT_AGG);
    let entries = format!(
        "coalesce(list(struct_pack(key := CAST({key} AS VARCHAR), value := CAST({value} AS JSON))) \
         FILTER (WHERE {key} IS NOT NULL AND {value} IS NOT NULL){over}, [])"
    );
    bound(
        &entries,
        "p",
        &format!(
            "(CASE WHEN len(list_distinct(list_transform(p, lambda e: e['key']))) <> len(p) THEN {} \
             WHEN list_bool_or(list_transform(p, lambda e: json_type(e['value']) = 'NULL')) THEN {} \
             ELSE {} END)",
            json_error(OBJECT_KEY_EXISTS, KEY_EXISTS_MESSAGE),
            fidelity_error("OBJECT_AGG of a JSON null value, which no recording shows"),
            sorted_object("p")
        ),
    )
}
