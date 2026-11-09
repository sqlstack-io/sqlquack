//! `json_kind_cast`: a VARIANT cast to OBJECT or ARRAY.

use sqlquack_diagnostics::failures::constants::VARIANT_NOT_CAST;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::{ARRAY_KIND, JSON_KIND_CAST};
use crate::variants::_helpers::objects::{bound, json_error};

/// The VARIANT when it holds `kind`; another value raises for OBJECT and is wrapped for ARRAY.
pub fn json_kind_cast(sql: &str, kind: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_KIND_CAST);
    let mismatch = match kind {
        ARRAY_KIND => "CAST(json_array(j) AS JSON)".to_owned(),
        _ => json_error(
            VARIANT_NOT_CAST,
            &format!("Failed to cast variant value to {kind}"),
        ),
    };
    bound(
        sql,
        "j",
        &format!(
            "(CASE WHEN j IS NULL OR json_type(j) = 'NULL' THEN NULL \
             WHEN json_type(j) <> '{kind}' THEN {mismatch} ELSE j END)"
        ),
    )
}
