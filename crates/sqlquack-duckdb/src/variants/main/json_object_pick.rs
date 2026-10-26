//! `json_object_pick`: OBJECT_PICK of literal keys or of an array of strings.

use sqlquack_core::failures::constants::VARIANT_NOT_CAST;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_OBJECT_PICK, NOT_AN_OBJECT_MESSAGE};
use crate::semi_structured::models::PickedKeys;
use crate::variants::_helpers::objects::{
    bound, fidelity_error, json_error, key_list, object_entries, sorted_object, string_elements,
};

/// The object with only the keys named, matched exactly; NULL or JSON null give NULL.
pub fn json_object_pick(sql: &str, keys: &PickedKeys, log: &ProvenanceLog) -> String {
    log.building_block(JSON_OBJECT_PICK);
    let (array, keys_sql, keys_check) = match keys {
        PickedKeys::Literal(keys) => (
            "CAST(NULL AS JSON)".to_owned(),
            key_list(keys),
            String::new(),
        ),
        PickedKeys::Array(array) => (
            format!("CAST({array} AS JSON)"),
            "CAST(p['k'] AS VARCHAR[])".to_owned(),
            format!(
                " WHEN p['k'] IS NULL THEN NULL WHEN json_type(p['k']) <> 'ARRAY' OR NOT {} THEN \
                 CAST({} AS JSON)",
                string_elements("p['k']"),
                fidelity_error("OBJECT_PICK of an array holding other than strings")
            ),
        ),
    };
    bound(
        &format!("struct_pack(o := CAST({sql} AS JSON), k := {array})"),
        "p",
        &format!(
            "(CASE WHEN p['o'] IS NULL OR json_type(p['o']) = 'NULL' THEN NULL \
             WHEN json_type(p['o']) <> 'OBJECT' THEN {}{keys_check} ELSE {} END)",
            json_error(VARIANT_NOT_CAST, NOT_AN_OBJECT_MESSAGE),
            sorted_object(&format!(
                "list_filter({}, lambda e: list_contains({keys_sql}, e['key']))",
                object_entries("p['o']")
            ))
        ),
    )
}
