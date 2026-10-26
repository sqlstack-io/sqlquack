//! `varchar_length_try`: NULL for text longer than `max_length` characters, as a TRY_CAST to VARCHAR(n) gives.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::VARCHAR_LENGTH_TRY;

/// NULL for text longer than `max_length` characters, as a TRY_CAST to VARCHAR(n) gives.
pub fn varchar_length_try(sql: &str, max_length: u32, log: &ProvenanceLog) -> String {
    log.building_block(VARCHAR_LENGTH_TRY);
    format!("(CASE WHEN length({sql}) > {max_length} THEN NULL ELSE {sql} END)")
}
