//! `text_to_zoned`: text in the recorded spellings as the UTC wall time of the instant it names.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    DATE_TEXT_SPELLING, ISO_TIMESTAMP_PATTERN, TEXT_TO_ZONED, ZONE_OFFSET_LIMITS,
    ZONE_OFFSET_PATTERN,
};
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::text::{bound, whitespace_trimmed};
use crate::temporal::constants::NANOS_PER_MINUTE;
use crate::temporal::main::text_to_timestamp::text_to_timestamp;

/// A nanosecond UTC wall time: the text's wall time less its offset; text without one is UTC.
pub fn text_to_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    log.building_block(TEXT_TO_ZONED);
    let sql = "z";
    let trimmed = whitespace_trimmed(sql);
    let wall = text_to_timestamp(sql, (on_failure, false), log);
    let zone = format!("regexp_extract({trimmed}, '^{ISO_TIMESTAMP_PATTERN}$', 6)");
    let part = |group: u8| {
        format!("CAST(regexp_extract({zone}, '{ZONE_OFFSET_PATTERN}', {group}) AS INTEGER)")
    };
    let (hours, minutes) = (part(2), part(3));
    let (largest_hour, largest_minute) = ZONE_OFFSET_LIMITS;
    let unrecorded = raised(&format!(
        "{DATE_TEXT_SPELLING}: an offset beyond the ones Snowflake was recorded reading"
    ));
    let offset = format!(
        "(CASE WHEN NOT regexp_matches({zone}, '[0-9]') THEN 0 \
         WHEN {hours} > {largest_hour} OR {minutes} > {largest_minute} THEN CAST({unrecorded} AS INTEGER) \
         WHEN regexp_extract({zone}, '{ZONE_OFFSET_PATTERN}', 1) = '-' THEN -({hours} * 60 + {minutes}) \
         ELSE {hours} * 60 + {minutes} END)"
    );
    let body = format!(
        "(CASE WHEN {sql} IS NULL THEN NULL ELSE \
         make_timestamp_ns(epoch_ns({wall}) - CAST({offset} AS BIGINT) * {NANOS_PER_MINUTE}) END)"
    );
    bound(value, sql, &body)
}
