//! `truncate_timestamp_to_date`: a naive timestamp's DATE, truncating toward the past.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{DUCKDB_FRACTIONAL_PRECISION, TRUNCATE_TIMESTAMP};
use crate::temporal::main::truncate_timestamp::truncate_timestamp;

/// The naive timestamp at `from` digits as its DATE, truncating toward the past.
pub fn truncate_timestamp_to_date(sql: &str, from: u8, log: &ProvenanceLog) -> String {
    log.building_block(TRUNCATE_TIMESTAMP);
    let microseconds = truncate_timestamp(sql, (from, DUCKDB_FRACTIONAL_PRECISION), log);
    format!("CAST({microseconds} AS DATE)")
}
