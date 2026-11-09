//! `variant_json_leaf`: a JSON-held value, such as parsed text, inside a VARIANT-held value.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::{bound, fidelity_error};
use crate::held::constants::{HELD_RETYPED_NUMBER_PATTERN, VARIANT_JSON_LEAF};
use crate::semi_structured::constants::JSON_STRING_PATTERN;

/// The JSON `sql`, raising for a number DuckDB's VARIANT would retype, as Snowflake keeps it exact.
pub fn variant_json_leaf(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_JSON_LEAF);
    bound(
        &format!("CAST({sql} AS JSON)"),
        "held_json",
        &format!(
            "(CASE WHEN regexp_matches(regexp_replace(CAST(held_json AS VARCHAR), '{}', '\"\"', \
             'g'), '{HELD_RETYPED_NUMBER_PATTERN}') THEN CAST({} AS JSON) ELSE held_json END)",
            JSON_STRING_PATTERN.replace('\'', "''"),
            fidelity_error("a JSON number beside a FLOAT in one VARIANT, which would be retyped")
        ),
    )
}
