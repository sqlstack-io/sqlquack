//! `time_text`: a TIME as Snowflake's default text, `HH24:MI:SS`.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::times::constants::TIME_TEXT;

/// Hours, minutes and seconds, without the fraction.
pub fn time_text(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(TIME_TEXT);
    format!("strftime(make_timestamp_ns(epoch_ns({sql})), '%H:%M:%S')")
}
