//! `time_diff_value`: clock unit boundaries crossed between two TIMEs.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::times::constants::{NANOS_PER_SECOND, TIME_DIFF};

/// Whole `unit`s from midnight to `end` less those to `start`.
pub fn time_diff_value(
    (start, end): (&str, &str),
    unit: &str,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(TIME_DIFF);
    let divisor = match unit {
        "SECOND" => NANOS_PER_SECOND,
        "MILLISECOND" => NANOS_PER_SECOND / 1000,
        other => return Err(format!("DATEDIFF in {other} of TIMEs has no recording")),
    };
    Ok(format!(
        "(epoch_ns({end}) // {divisor} - epoch_ns({start}) // {divisor})"
    ))
}
