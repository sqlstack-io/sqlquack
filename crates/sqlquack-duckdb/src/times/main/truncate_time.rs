//! `truncate_time`: a TIME cut to fewer fractional digits.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::NANOSECOND_DIGITS;
use crate::times::_helpers::nanos::{time_of_nanos, truncated_nanos};
use crate::times::constants::TRUNCATE_TIME;

/// The TIME_NS with digits past `precision` dropped.
pub fn truncate_time(sql: &str, precision: u8, log: &ProvenanceLog) -> String {
    log.building_block(TRUNCATE_TIME);
    if precision >= NANOSECOND_DIGITS {
        return sql.to_owned();
    }
    let body = time_of_nanos(&truncated_nanos("epoch_ns(t)", precision));
    format!("list_transform([{sql}], lambda t: {body})[1]")
}
