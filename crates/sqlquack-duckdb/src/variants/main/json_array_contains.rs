//! `json_array_contains`: ARRAY_CONTAINS of a VARIANT value in a VARIANT array.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_ARRAY_CONTAINS;
use crate::variants::_helpers::objects::{bound, fidelity_error};

/// Whether the array holds the value, compared as JSON text; decimal or nested values raise.
pub fn json_array_contains((value, array): (&str, &str), log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_CONTAINS);
    let elements = "json_extract(p['a'], '$[*]')";
    bound(
        &format!("struct_pack(v := CAST({value} AS JSON), a := CAST({array} AS JSON))"),
        "p",
        &format!(
            "(CASE WHEN p['a'] IS NULL OR p['v'] IS NULL THEN NULL \
             WHEN json_type(p['a']) <> 'ARRAY' THEN CAST({} AS BOOLEAN) \
             WHEN json_type(p['v']) IN ('OBJECT', 'ARRAY', 'DOUBLE') \
             OR list_contains(list_transform({elements}, lambda e: json_type(e)), 'DOUBLE') \
             THEN CAST({} AS BOOLEAN) \
             ELSE list_contains(CAST({elements} AS VARCHAR[]), CAST(p['v'] AS VARCHAR)) END)",
            fidelity_error("ARRAY_CONTAINS of a value other than an array"),
            fidelity_error("ARRAY_CONTAINS comparing decimal or nested values")
        ),
    )
}
