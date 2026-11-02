//! `json_flatten`: FLATTEN's rows for a VARIANT, as a DuckDB subquery.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::fidelity_error;
use crate::semi_structured::constants::{JSON_FLATTEN, PLAIN_KEY_PATTERN};
use crate::semi_structured::models::FlattenMode;

/// Each element or key `mode` takes; `outer` adds the recorded row for an input with none.
pub fn json_flatten(sql: &str, (mode, outer): (FlattenMode, bool), log: &ProvenanceLog) -> String {
    log.building_block(JSON_FLATTEN);
    let kinds = match mode {
        FlattenMode::Both => "'ARRAY', 'OBJECT'",
        FlattenMode::Array => "'ARRAY'",
        FlattenMode::Object => "'OBJECT'",
    };
    let taken = format!("json_type(_FLATTEN_INPUT.j) IN ({kinds})");
    let elements = format!(
        "SELECT CASE WHEN json_type(_FLATTEN_INPUT.j) = 'ARRAY' THEN CAST(_FLATTEN_ROW.key AS DECIMAL(38,0)) END AS \"INDEX\", \
         CASE WHEN json_type(_FLATTEN_INPUT.j) = 'OBJECT' THEN _FLATTEN_ROW.key END AS \"KEY\", \
         CASE WHEN json_type(_FLATTEN_INPUT.j) = 'ARRAY' THEN '[' || _FLATTEN_ROW.key || ']' \
         WHEN regexp_full_match(_FLATTEN_ROW.key, '{PLAIN_KEY_PATTERN}') THEN _FLATTEN_ROW.key \
         ELSE CAST({} AS VARCHAR) END AS \"PATH\", \
         _FLATTEN_ROW.value AS \"VALUE\", _FLATTEN_INPUT.j AS \"THIS\" \
         FROM (SELECT {sql} AS j) AS _FLATTEN_INPUT, json_each(_FLATTEN_INPUT.j) AS _FLATTEN_ROW \
         WHERE {taken}",
        fidelity_error("a FLATTEN path through a key that is not a plain name")
    );
    match outer {
        false => elements,
        true => format!(
            "{elements} UNION ALL SELECT CAST(NULL AS DECIMAL(38,0)) AS \"INDEX\", \
             CAST(NULL AS VARCHAR) AS \"KEY\", CASE WHEN {taken} THEN '' END AS \"PATH\", \
             CAST(NULL AS JSON) AS \"VALUE\", CASE WHEN {taken} THEN _FLATTEN_INPUT.j END AS \"THIS\" \
             FROM (SELECT {sql} AS j) AS _FLATTEN_INPUT \
             WHERE coalesce(NOT ({taken}) OR json_array_length(_FLATTEN_INPUT.j) + \
             coalesce(len(json_keys(_FLATTEN_INPUT.j)), 0) = 0, TRUE)"
        ),
    }
}
