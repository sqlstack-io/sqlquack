//! `json_get_computed`: GET with an index or key computed per row.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{JSON_GET_COMPUTED, PATH_READ_KEYS};

/// An array element at a non-negative integer `step`, or an object's value at exactly a text key.
pub fn json_get_computed(base: &str, (step, index): (&str, bool), log: &ProvenanceLog) -> String {
    log.building_block(JSON_GET_COMPUTED);
    let found = match index {
        true => "CASE WHEN json_type(q['v']) = 'ARRAY' AND q['s'] >= 0 \
                 AND q['s'] < json_array_length(q['v']) \
                 THEN json_extract(q['v'], CAST(q['s'] AS BIGINT)) END"
            .to_owned(),
        false => {
            let path_keys: Vec<String> = PATH_READ_KEYS
                .iter()
                .map(|key| format!("'{key}'"))
                .collect();
            format!(
                "CASE WHEN json_type(q['v']) <> 'OBJECT' THEN NULL \
                 WHEN q['s'] IN ({}) THEN list_transform([CAST(q['v'] AS MAP(VARCHAR, JSON))], \
                 lambda m: CASE WHEN map_contains(m, q['s']) \
                 THEN coalesce(map_extract_value(m, q['s']), CAST('null' AS JSON)) END)[1] \
                 ELSE json_extract(q['v'], '$.\"' || replace(replace(q['s'], '\\', '\\\\'), \
                 '\"', '\\\"') || '\"') END",
                path_keys.join(", "),
            )
        }
    };
    let step = match index {
        true => format!("CAST({step} AS HUGEINT)"),
        false => format!("CAST({step} AS VARCHAR)"),
    };
    format!(
        "list_transform([struct_pack(v := CAST({base} AS JSON), s := {step})], lambda q: {found})[1]"
    )
}
