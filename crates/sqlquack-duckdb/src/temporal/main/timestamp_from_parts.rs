//! `timestamp_from_parts`: a nanosecond timestamp from a date and nanoseconds into it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::temporal::constants::TIMESTAMP_FROM_PARTS;

/// The TIMESTAMP_NS `nanos` nanoseconds after midnight of `date`, which may pass other days.
pub fn timestamp_from_parts(date: &str, nanos: &str, log: &ProvenanceLog) -> String {
    log.building_block(TIMESTAMP_FROM_PARTS);
    format!("make_timestamp_ns(epoch_ns(CAST({date} AS TIMESTAMP_NS)) + {nanos})")
}
