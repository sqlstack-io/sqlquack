//! `utc_wall_time`: a zoned instant as its naive wall time at +00:00.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::UTC_WALL_TIME;

/// The wall time in UTC of the TIMESTAMPTZ `sql`, a TIMESTAMP.
pub fn utc_wall_time(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(UTC_WALL_TIME);
    format!("timezone('UTC', {sql})")
}
