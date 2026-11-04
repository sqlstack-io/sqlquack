//! `json_is_number`: IS_DECIMAL and IS_INTEGER of a VARIANT held as JSON.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_IS_NUMBER;
use crate::variants::_helpers::objects::bound;

/// Whether the VARIANT holds a number, or with `whole` an integer, as TYPEOF types it; SQL NULL
/// stays NULL.
pub fn json_is_number(sql: &str, whole: bool, log: &ProvenanceLog) -> String {
    log.building_block(JSON_IS_NUMBER);
    let fraction = match whole {
        true => "NOT contains(CAST(j AS VARCHAR), '.')",
        false => "true",
    };
    bound(
        sql,
        "j",
        &format!(
            "(CASE WHEN j IS NULL THEN NULL WHEN json_type(j) IN ('BIGINT', 'UBIGINT') THEN true \
             WHEN json_type(j) = 'DOUBLE' THEN {fraction} ELSE false END)"
        ),
    )
}
