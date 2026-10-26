//! `epoch_to_timestamp`: an integer count since the epoch as a naive timestamp.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    DUCKDB_FRACTIONAL_PRECISION, EPOCH_TO_TIMESTAMP, NANOSECOND_DIGITS,
};

/// `sql` units of 10^-`scale` seconds since the epoch, held at microseconds or nanoseconds.
pub fn epoch_to_timestamp(sql: &str, scale: u8, log: &ProvenanceLog) -> String {
    log.building_block(EPOCH_TO_TIMESTAMP);
    match scale <= DUCKDB_FRACTIONAL_PRECISION {
        true => format!(
            "make_timestamp(CAST({sql} AS BIGINT) * {})",
            10_i64.pow(u32::from(DUCKDB_FRACTIONAL_PRECISION - scale))
        ),
        false => format!(
            "make_timestamp_ns(CAST({sql} AS BIGINT) * {})",
            10_i64.pow(u32::from(NANOSECOND_DIGITS - scale))
        ),
    }
}
