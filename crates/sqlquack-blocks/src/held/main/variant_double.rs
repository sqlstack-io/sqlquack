//! `variant_double`: a VARIANT-held value cast to FLOAT.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::{bound, fidelity_error};
use crate::held::constants::VARIANT_DOUBLE;

/// The VARIANT-held `sql` as a FLOAT, nulls as NULL; values other than numbers raise.
pub fn variant_double(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_DOUBLE);
    bound(
        &format!("CAST({sql} AS JSON)"),
        "held_value",
        &format!(
            "(CASE WHEN held_value IS NULL OR json_type(held_value) = 'NULL' THEN NULL \
             WHEN json_type(held_value) IN ('DOUBLE', 'BIGINT', 'UBIGINT') \
             THEN CAST(held_value AS DOUBLE) ELSE CAST({} AS DOUBLE) END)",
            fidelity_error("a VARIANT other than a number cast to FLOAT")
        ),
    )
}
