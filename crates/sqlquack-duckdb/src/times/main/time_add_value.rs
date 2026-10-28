//! `time_add_value`: a TIME moved by whole hours or minutes, wrapping at midnight.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::temporal::constants::NANOS_PER_MINUTE;
use crate::times::_helpers::nanos::time_of_nanos;
use crate::times::constants::{NANOS_PER_DAY, TIME_ADD};

/// The TIME plus `amount` units, modulo a day.
pub fn time_add_value(
    sql: &str,
    (unit, amount): (&str, &str),
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(TIME_ADD);
    let step = match unit {
        "HOUR" => NANOS_PER_MINUTE * 60,
        "MINUTE" => NANOS_PER_MINUTE,
        other => return Err(format!("DATEADD of {other} to a TIME has no recording")),
    };
    let moved = format!(
        "((epoch_ns({sql}) + CAST({amount} AS HUGEINT) * {step}) % {NANOS_PER_DAY} + {NANOS_PER_DAY}) % {NANOS_PER_DAY}"
    );
    Ok(time_of_nanos(&format!("CAST({moved} AS BIGINT)")))
}
