//! `json_flatten`: FLATTEN's rows for a VARIANT, as a DuckDB subquery.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::fidelity_error;
use crate::semi_structured::constants::{JSON_FLATTEN, PLAIN_KEY_PATTERN};

/// INDEX, KEY, PATH, VALUE and THIS for each element or key; scalars, JSON null and NULL give none.
pub fn json_flatten(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_FLATTEN);
    format!(
        "SELECT CASE WHEN json_type(_FLATTEN_INPUT.j) = 'ARRAY' THEN CAST(_FLATTEN_ROW.key AS DECIMAL(38,0)) END AS \"INDEX\", \
         CASE WHEN json_type(_FLATTEN_INPUT.j) = 'OBJECT' THEN _FLATTEN_ROW.key END AS \"KEY\", \
         CASE WHEN json_type(_FLATTEN_INPUT.j) = 'ARRAY' THEN '[' || _FLATTEN_ROW.key || ']' \
         WHEN regexp_full_match(_FLATTEN_ROW.key, '{PLAIN_KEY_PATTERN}') THEN _FLATTEN_ROW.key \
         ELSE CAST({} AS VARCHAR) END AS \"PATH\", \
         _FLATTEN_ROW.value AS \"VALUE\", _FLATTEN_INPUT.j AS \"THIS\" \
         FROM (SELECT {sql} AS j) AS _FLATTEN_INPUT, json_each(_FLATTEN_INPUT.j) AS _FLATTEN_ROW \
         WHERE json_type(_FLATTEN_INPUT.j) IN ('ARRAY', 'OBJECT')",
        fidelity_error("a FLATTEN path through a key that is not a plain name")
    )
}
