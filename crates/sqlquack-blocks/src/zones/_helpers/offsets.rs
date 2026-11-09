//! A text's offset applied to its wall time, giving the instant as UTC wall time.

use crate::blocks::constants::{
    CONVERSION_WHITESPACE, DATE_TEXT_SPELLING, ISO_TIMESTAMP_PATTERN, ZONE_OFFSET_LIMITS,
    ZONE_OFFSET_PATTERN,
};
use crate::raises::main::raised::raised;
use crate::temporal::constants::NANOS_PER_MINUTE;

/// The lambda parameter holding the offset's sign, hours and minutes.
const OFFSET: &str = "zoned_offset";

/// The text `sql`'s instant: its parsed `wall` time less the offset it spells; no offset is UTC.
pub(crate) fn zoned_instant(sql: &str, wall: &str) -> String {
    let trimmed = format!("trim({sql}, {CONVERSION_WHITESPACE})");
    let zone = format!(
        "regexp_extract(regexp_extract({trimmed}, '^{ISO_TIMESTAMP_PATTERN}$', 6), \
         '{ZONE_OFFSET_PATTERN}', ['sign', 'hours', 'minutes'])"
    );
    let (hours, minutes) = (
        format!("CAST({OFFSET}['hours'] AS INTEGER)"),
        format!("CAST({OFFSET}['minutes'] AS INTEGER)"),
    );
    let (largest_hour, largest_minute) = ZONE_OFFSET_LIMITS;
    let unrecorded = raised(&format!(
        "{DATE_TEXT_SPELLING}: an offset beyond the ones Snowflake was recorded reading"
    ));
    let offset = format!(
        "list_transform([{zone}], lambda {OFFSET}: CASE WHEN {OFFSET}['hours'] = '' THEN 0 \
         WHEN {hours} > {largest_hour} OR {minutes} > {largest_minute} THEN CAST({unrecorded} AS INTEGER) \
         WHEN {OFFSET}['sign'] = '-' THEN -({hours} * 60 + {minutes}) \
         ELSE {hours} * 60 + {minutes} END)[1]"
    );
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL ELSE \
         make_timestamp_ns(epoch_ns({wall}) - CAST({offset} AS BIGINT) * {NANOS_PER_MINUTE}) END)"
    )
}
