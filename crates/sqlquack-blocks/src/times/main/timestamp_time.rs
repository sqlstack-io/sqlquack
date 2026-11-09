//! `timestamp_time`: the time of day of a naive timestamp, as a TIME.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::times::_helpers::nanos::time_of_nanos;
use crate::times::constants::{MICROS_PER_DAY, NANOS_PER_DAY, TIMESTAMP_TIME};

/// The wall-clock time of `sql`, a DuckDB TIMESTAMP when `nanoseconds` is false, else TIMESTAMP_NS.
pub fn timestamp_time(sql: &str, nanoseconds: bool, log: &ProvenanceLog) -> String {
    log.building_block(TIMESTAMP_TIME);
    let nanos = match nanoseconds {
        true => {
            format!("((epoch_ns({sql}) % {NANOS_PER_DAY} + {NANOS_PER_DAY}) % {NANOS_PER_DAY})")
        }
        false => format!(
            "((epoch_us({sql}) % {MICROS_PER_DAY} + {MICROS_PER_DAY}) % {MICROS_PER_DAY} * 1000)"
        ),
    };
    time_of_nanos(&nanos)
}
