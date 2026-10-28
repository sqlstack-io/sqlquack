//! `json_get_computed`: GET with an index or key computed per row.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::{JSON_FIDELITY, JSON_GET_COMPUTED, PATH_READ_KEYS};

/// The array element at a non-negative integer `step`, or the object value at exactly a text key.
pub fn json_get_computed(base: &str, (step, index): (&str, bool), log: &ProvenanceLog) -> String {
    log.building_block(JSON_GET_COMPUTED);
    let found = match index {
        true => "CASE WHEN json_type(q['v']) = 'ARRAY' AND q['s'] >= 0 \
                 THEN json_extract(q['v'], CAST(q['s'] AS BIGINT)) END"
            .to_owned(),
        false => {
            let path_keys: Vec<String> = PATH_READ_KEYS
                .iter()
                .map(|key| format!("'{key}'"))
                .collect();
            format!(
                "CASE WHEN json_type(q['v']) <> 'OBJECT' THEN NULL \
                 WHEN q['s'] IN ({}) THEN CAST({} AS JSON) \
                 ELSE json_extract(q['v'], '$.\"' || replace(replace(q['s'], '\\', '\\\\'), \
                 '\"', '\\\"') || '\"') END",
                path_keys.join(", "),
                raised(&format!(
                    "{JSON_FIDELITY}: GET of a key DuckDB reads only as a path, empty or '*'"
                ))
            )
        }
    };
    let step = match index {
        true => format!("CAST({step} AS BIGINT)"),
        false => format!("CAST({step} AS VARCHAR)"),
    };
    format!(
        "list_transform([struct_pack(v := CAST({base} AS JSON), s := {step})], lambda q: {found})[1]"
    )
}
