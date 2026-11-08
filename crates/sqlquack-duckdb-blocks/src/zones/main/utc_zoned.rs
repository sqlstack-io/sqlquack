//! `utc_zoned`: a naive wall time as the instant it names at +00:00.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::zones::constants::UTC_ZONED;

/// The TIMESTAMPTZ at +00:00 whose UTC wall time is the TIMESTAMP `sql`.
pub fn utc_zoned(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(UTC_ZONED);
    format!("timezone('UTC', CAST({sql} AS TIMESTAMP))")
}
