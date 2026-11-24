//! `json_object_construct`: OBJECT_CONSTRUCT and OBJECT_CONSTRUCT_KEEP_NULL.

use sqlquack_diagnostics::failures::constants::OBJECT_KEY_EXISTS;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_OBJECT_CONSTRUCT, KEY_EXISTS_MESSAGE};
use crate::variants::_helpers::objects::{bound, json_error, sorted_object};

/// An object of `(key, JSON value)` pairs: NULL keys skipped, NULL values skipped or kept as null.
/// With `constant` keys, distinct literals, no key can repeat or be NULL, so none is checked.
pub fn json_object_construct(
    pairs: &[(String, String)],
    (keep_null, constant): (bool, bool),
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_OBJECT_CONSTRUCT);
    if pairs.is_empty() {
        return "CAST('{}' AS JSON)".to_owned();
    }
    if constant {
        let entries: Vec<String> = pairs
            .iter()
            .map(|(key, value)| match keep_null {
                true => format!(
                    "struct_pack(key := {key}, value := coalesce(CAST({value} AS JSON), \\
                     CAST('null' AS JSON)))"
                ),
                false => format!("struct_pack(key := {key}, value := CAST({value} AS JSON))"),
            })
            .collect();
        let listed = format!("[{}]", entries.join(", "));
        return sorted_object(&match keep_null {
            true => listed,
            false => format!("list_filter({listed}, lambda e: e['value'] IS NOT NULL)"),
        });
    }
    let entries: Vec<String> = pairs
        .iter()
        .map(|(key, value)| {
            format!("struct_pack(key := CAST({key} AS VARCHAR), value := CAST({value} AS JSON))")
        })
        .collect();
    let kept = match keep_null {
        true => "list_transform(list_filter(p, lambda e: e['key'] IS NOT NULL), lambda e: \
                 struct_pack(key := e['key'], value := coalesce(e['value'], CAST('null' AS JSON))))"
            .to_owned(),
        false => {
            "list_filter(p, lambda e: e['key'] IS NOT NULL AND e['value'] IS NOT NULL)".to_owned()
        }
    };
    bound(
        &format!("[{}]", entries.join(", ")),
        "p",
        &format!(
            "(CASE WHEN len(list_distinct(list_transform(list_filter(p, lambda e: e['key'] IS NOT NULL), \
             lambda e: e['key']))) <> len(list_filter(p, lambda e: e['key'] IS NOT NULL)) THEN {} \
             ELSE {} END)",
            json_error(OBJECT_KEY_EXISTS, KEY_EXISTS_MESSAGE),
            sorted_object(&kept)
        ),
    )
}
