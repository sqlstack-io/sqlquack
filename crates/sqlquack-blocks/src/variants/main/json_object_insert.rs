//! `json_object_insert`: OBJECT_INSERT of one literal key.

use sqlquack_diagnostics::failures::constants::{OBJECT_KEY_EXISTS, VARIANT_NOT_CAST};
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{
    JSON_OBJECT_INSERT, KEY_EXISTS_MESSAGE, NOT_AN_OBJECT_MESSAGE,
};
use crate::variants::_helpers::objects::{
    bound, json_error, key_list, object_entries, sorted_object,
};

/// The object with `key` set to JSON `value`; a NULL value keeps it, or removes it when updating.
pub fn json_object_insert(
    (sql, value): (&str, &str),
    (key, update): (&str, bool),
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_OBJECT_INSERT);
    let keys = key_list(&[key.to_owned()]);
    let nulled = match update {
        true => sorted_object(&format!(
            "list_filter({}, lambda e: NOT list_contains({keys}, e['key']))",
            object_entries("a['o']")
        )),
        false => sorted_object(&object_entries("a['o']")),
    };
    let existing = match update {
        true => String::new(),
        false => format!(
            " WHEN list_has_any(json_keys(a['o']), {keys}) THEN {}",
            json_error(OBJECT_KEY_EXISTS, KEY_EXISTS_MESSAGE)
        ),
    };
    bound(
        &format!("struct_pack(o := {sql}, v := CAST({value} AS JSON))"),
        "a",
        &format!(
            "(CASE WHEN a['o'] IS NULL OR json_type(a['o']) = 'NULL' THEN NULL WHEN json_type(a['o']) <> 'OBJECT' THEN {} \
             WHEN a['v'] IS NULL THEN {nulled}{existing} ELSE {} END)",
            json_error(VARIANT_NOT_CAST, NOT_AN_OBJECT_MESSAGE),
            sorted_object(&format!(
                "list_append(list_filter({}, lambda e: NOT list_contains({keys}, e['key'])), \
                 struct_pack(key := {keys}[1], value := a['v']))",
                object_entries("a['o']")
            ))
        ),
    )
}
