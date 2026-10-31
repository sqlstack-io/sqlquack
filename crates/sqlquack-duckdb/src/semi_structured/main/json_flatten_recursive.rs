//! `json_flatten_recursive`: FLATTEN's rows at every depth of a VARIANT, as a DuckDB subquery.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::fidelity_error;
use crate::semi_structured::constants::{
    BRACKETED_KEY, ELEMENT_STEP, JSON_FLATTEN_RECURSIVE, KEY_STEP, PLAIN_KEY_PATTERN,
};
use crate::semi_structured::models::FlattenMode;

/// INDEX, KEY, PATH, VALUE and THIS at every depth, keys in byte order; a scalar gives itself.
pub fn json_flatten_recursive(sql: &str, mode: FlattenMode, log: &ProvenanceLog) -> String {
    log.building_block(JSON_FLATTEN_RECURSIVE);
    let row = "_FLATTEN_ROW";
    let input = "_FLATTEN_INPUT.j";
    let parent_type = format!("json_type(json_extract({input}, {row}.path))");
    let descended = match mode {
        FlattenMode::Both => "TRUE".to_owned(),
        FlattenMode::Object => format!("regexp_full_match({row}.fullkey, '\\$({KEY_STEP})*')"),
        FlattenMode::Array => format!("regexp_full_match({row}.fullkey, '\\$({ELEMENT_STEP})*')"),
    };
    let scalar_root = match mode {
        FlattenMode::Both => "TRUE".to_owned(),
        FlattenMode::Object | FlattenMode::Array => format!(
            "CAST({} AS BOOLEAN)",
            fidelity_error("a recursive FLATTEN with a MODE over a scalar")
        ),
    };
    let written = format!(r"\.{PLAIN_KEY_PATTERN}|{BRACKETED_KEY}|{ELEMENT_STEP}");
    format!(
        "SELECT CASE WHEN {row}.parent IS NOT NULL AND {parent_type} = 'ARRAY' \
         THEN CAST({row}.key AS DECIMAL(38,0)) END AS \"INDEX\", \
         CASE WHEN {row}.parent IS NOT NULL AND {parent_type} = 'OBJECT' THEN {row}.key END AS \"KEY\", \
         CASE WHEN regexp_full_match({row}.fullkey, '\\$({written})*') \
         THEN regexp_replace(regexp_replace(regexp_replace({row}.fullkey, \
         '\\.\"({PLAIN_KEY_PATTERN})\"', '.\\1', 'g'), '\\.\"([^\"]*)\"', '[''\\1'']', 'g'), \
         '^\\$\\.?', '') \
         ELSE CAST({raise} AS VARCHAR) END AS \"PATH\", \
         {row}.value AS \"VALUE\", \
         CASE WHEN {row}.parent IS NULL THEN {row}.value ELSE json_extract({input}, {row}.path) END AS \"THIS\" \
         FROM (SELECT {sql} AS j) AS _FLATTEN_INPUT, json_tree({input}) AS {row} \
         WHERE CASE WHEN {row}.parent IS NULL \
         THEN CASE WHEN json_type({input}) IN ('ARRAY', 'OBJECT') THEN FALSE ELSE {scalar_root} END \
         ELSE {descended} END \
         ORDER BY list_transform(regexp_extract_all({row}.fullkey, '{KEY_STEP}|{ELEMENT_STEP}'), \
         lambda step: CASE WHEN step[1] = '[' THEN '1' || lpad(step[2:-2], 20, '0') \
         WHEN step[2] <> '\"' THEN '0' || step[2:] \
         WHEN NOT contains(step, '\\') THEN '0' || step[3:-2] \
         ELSE CAST({unordered} AS VARCHAR) END)",
        raise = fidelity_error("a FLATTEN path through a key that is not a plain name"),
        unordered = fidelity_error("a recursive FLATTEN through a key holding an escape"),
    )
}
