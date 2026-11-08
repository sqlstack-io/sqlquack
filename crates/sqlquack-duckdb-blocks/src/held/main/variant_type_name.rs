//! `variant_type_name`: Snowflake's TYPEOF of a VARIANT-held value.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::bound;
use crate::held::constants::VARIANT_TYPE_NAME;
use crate::variants::main::json_type_name::json_type_name;

/// TYPEOF of the VARIANT-held `sql`; with `typed`, strings that may be typed values raise.
pub fn variant_type_name(sql: &str, typed: bool, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_TYPE_NAME);
    bound(
        &format!("CAST({sql} AS JSON)"),
        "held_json",
        &format!(
            "(CASE WHEN json_type(held_json) = 'DOUBLE' THEN 'DOUBLE' ELSE {} END)",
            json_type_name("held_json", typed, log)
        ),
    )
}
