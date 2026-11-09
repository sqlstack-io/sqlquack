//! `inline_zoned`: `text_to_zoned` written out, for its helper macros' bodies and VARIANT reads.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    CONVERSION_WHITESPACE, DATE_TEXT_SPELLING, ISO_TIMESTAMP_PATTERN, ZONE_OFFSET_LIMITS,
    ZONE_OFFSET_PATTERN,
};
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::constants::NANOS_PER_MINUTE;
use crate::temporal::main::text_to_timestamp::text_to_timestamp;

/// `text_to_zoned` written out, as a helper macro's body and a VARIANT's string read it.
pub fn inline_zoned(value: &str, on_failure: TextFailure, log: &ProvenanceLog) -> String {
    let sql = "z";
    let trimmed = format!("trim({sql}, {CONVERSION_WHITESPACE})");
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
    format!("list_transform([{value}], lambda {sql}: {body})[1]")
}
