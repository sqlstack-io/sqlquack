//! `variant_double_leaf`: a FLOAT as a leaf of a VARIANT-held value.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::{bound, fidelity_error};
use crate::held::constants::VARIANT_DOUBLE_LEAF;

/// The FLOAT `sql` as JSON; NaN, the infinities and negative zero raise, being unrecorded.
pub fn variant_double_leaf(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_DOUBLE_LEAF);
    bound(
        &format!("CAST({sql} AS DOUBLE)"),
        "held_double",
        &format!(
            "(CASE WHEN isnan(held_double) OR isinf(held_double) \
             OR (held_double = 0 AND signbit(held_double)) THEN CAST({} AS JSON) \
             ELSE to_json(held_double) END)",
            fidelity_error("a FLOAT in a VARIANT that is NaN, infinite or negative zero")
        ),
    )
}
