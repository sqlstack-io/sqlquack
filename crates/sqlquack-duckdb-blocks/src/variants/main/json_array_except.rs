//! `json_array_except`: ARRAY_EXCEPT of two arrays of strings.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_ARRAY_EXCEPT;
use crate::variants::_helpers::objects::{bound, fidelity_error, string_elements};

/// The first array without one occurrence per occurrence in the second, in order; strings only.
pub fn json_array_except((left, right): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_EXCEPT);
    let kept = "list_transform(list_filter(range(1, len(p['l']) + 1), lambda i: \
         len(list_filter(p['l'][1:i], lambda x: x = p['l'][i])) > \
         len(list_filter(p['r'], lambda y: y = p['l'][i]))), lambda i: p['l'][i])";
    bound(
        &format!("struct_pack(a := CAST({left} AS JSON), b := CAST({right} AS JSON))"),
        "q",
        &format!(
            "(CASE WHEN q['a'] IS NULL OR q['b'] IS NULL THEN NULL \
             WHEN json_type(q['a']) <> 'ARRAY' OR json_type(q['b']) <> 'ARRAY' THEN {} \
             WHEN NOT ({} AND {}) THEN CAST({} AS JSON) \
             ELSE {} END)",
            fidelity_error("ARRAY_EXCEPT of a value other than an array"),
            string_elements("q['a']"),
            string_elements("q['b']"),
            fidelity_error("ARRAY_EXCEPT over elements other than strings"),
            bound(
                "struct_pack(l := CAST(q['a'] AS VARCHAR[]), r := CAST(q['b'] AS VARCHAR[]))",
                "p",
                &format!("CAST(to_json({kept}) AS JSON)")
            )
        ),
    )
}
