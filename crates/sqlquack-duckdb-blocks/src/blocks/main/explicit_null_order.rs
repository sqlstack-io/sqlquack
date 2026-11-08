//! `explicit_null_order`: a sort key with NULL placement spelled out; Snowflake's default puts NULLs high.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::EXPLICIT_NULL_ORDER;

/// A sort key with NULL placement spelled out; Snowflake's default puts NULLs high.
pub fn explicit_null_order(
    sql: &str,
    descending: bool,
    nulls_first: Option<bool>,
    log: &ProvenanceLog,
) -> String {
    log.building_block(EXPLICIT_NULL_ORDER);
    let direction = if descending { "DESC" } else { "ASC" };
    let nulls = if nulls_first.unwrap_or(descending) {
        "NULLS FIRST"
    } else {
        "NULLS LAST"
    };
    format!("{sql} {direction} {nulls}")
}
