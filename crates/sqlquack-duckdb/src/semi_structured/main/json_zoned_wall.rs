//! `json_zoned_wall`: a VARIANT cast to TIMESTAMP_TZ, as the UTC wall time of its instant.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::models::TextFailure;
use crate::semi_structured::_helpers::guards::{bound, fidelity_error};
use crate::semi_structured::constants::{JSON_ZONED_WALL, NANOS_PER_SECOND};
use crate::temporal::main::text_to_zoned::text_to_zoned;

/// Strings as zoned text, whole numbers as epoch seconds, JSON null as NULL; others raise.
pub fn json_zoned_wall(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ZONED_WALL);
    let refused = |kind: &str| {
        format!(
            "CAST({} AS TIMESTAMP_NS)",
            fidelity_error(&format!("a VARIANT holding {kind} cast to TIMESTAMP_TZ"))
        )
    };
    let body = format!(
        "(CASE WHEN json_type(j) = 'NULL' THEN NULL \
         WHEN json_type(j) = 'VARCHAR' THEN {} \
         WHEN json_type(j) IN ('BIGINT', 'UBIGINT') \
         THEN make_timestamp_ns(CAST(CAST(j AS VARCHAR) AS BIGINT) * {NANOS_PER_SECOND}) \
         WHEN j IS NULL THEN NULL ELSE {} END)",
        text_to_zoned("json_extract_string(j, '$')", TextFailure::Variant, log),
        refused("a value other than a string or a whole number")
    );
    bound(sql, "j", &body)
}
