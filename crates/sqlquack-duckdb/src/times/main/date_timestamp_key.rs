//! `date_timestamp_key`: an order key comparing a DATE with a nanosecond timestamp, range safe.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::times::constants::{DATE_TIMESTAMP_KEY, DAY_NANOSECONDS};

/// Twice the days since 1970 of a DATE (`timestamp` false) or a TIMESTAMP_NS, plus one past midnight.
pub fn date_timestamp_key(sql: &str, timestamp: bool, log: &ProvenanceLog) -> String {
    log.building_block(DATE_TIMESTAMP_KEY);
    match timestamp {
        false => format!("(2 * ({sql} - DATE '1970-01-01'))"),
        true => format!(
            "(2 * (CAST({sql} AS DATE) - DATE '1970-01-01') \
             + CAST(epoch_ns({sql}) % {DAY_NANOSECONDS} <> 0 AS BIGINT))"
        ),
    }
}
