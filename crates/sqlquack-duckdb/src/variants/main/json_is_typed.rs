//! `json_is_typed`: IS_DATE, IS_TIMESTAMP_* and IS_TIME of a VARIANT held as JSON.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::main::raised::raised;
use crate::semi_structured::constants::{JSON_IS_TYPED, VARIANT_TYPED_VALUES};
use crate::variants::_helpers::objects::{bound, typed_text};

/// False, raising for a string in a typed value's VARIANT text, which may have been one.
pub fn json_is_typed(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_IS_TYPED);
    bound(
        sql,
        "j",
        &format!(
            "(CASE WHEN j IS NULL THEN NULL WHEN {} THEN {} ELSE false END)",
            typed_text("j"),
            raised(&format!(
                "{VARIANT_TYPED_VALUES}: a string that may be a typed value in Snowflake"
            ))
        ),
    )
}
