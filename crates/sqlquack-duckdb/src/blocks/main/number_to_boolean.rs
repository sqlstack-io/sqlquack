//! `number_to_boolean`: nUMBER to BOOLEAN: zero is false.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::BOOLEAN_NUMBER;

/// NUMBER to BOOLEAN: zero is false.
pub fn number_to_boolean(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(BOOLEAN_NUMBER);
    format!("({sql} <> 0)")
}
