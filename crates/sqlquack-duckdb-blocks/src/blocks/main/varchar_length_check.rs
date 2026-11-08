//! `varchar_length_check`: raises an error when text is longer than `max_length` characters, as a VARCHAR(n) cast does.

use sqlquack_core::failures::constants::STRING_TOO_LONG;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::tagged_error;
use crate::blocks::constants::VARCHAR_LENGTH_CHECK;

/// Raises an error when text is longer than `max_length` characters, as a VARCHAR(n) cast does.
pub fn varchar_length_check(sql: &str, max_length: u32, log: &ProvenanceLog) -> String {
    log.building_block(VARCHAR_LENGTH_CHECK);
    format!(
        "(CASE WHEN length({sql}) > {max_length} THEN {} ELSE {sql} END)",
        tagged_error(STRING_TOO_LONG, "String is too long and would be truncated")
    )
}
