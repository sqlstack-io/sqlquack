//! `null_propagating_concat`: `a || b || ...`, NULL when any operand is NULL.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::NULL_PROPAGATING_CONCAT;

/// `a || b || ...`, NULL when any operand is NULL.
pub fn null_propagating_concat(parts: &[String], log: &ProvenanceLog) -> String {
    log.building_block(NULL_PROPAGATING_CONCAT);
    format!("({})", parts.join(" || "))
}
