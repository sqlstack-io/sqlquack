//! `formatted_text_to_timestamp`: TRY_TO_TIMESTAMP_NTZ of text in one literal format.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{DATE_TEXT_SPELLING, FORMATTED_TEXT_TO_TIMESTAMP};
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::text::bound;
use crate::temporal::models::{TimestampField, TimestampFormat};

/// The timestamp `sql` spells in `format`: NULL if empty or impossible, raising if not so spelled.
pub fn formatted_text_to_timestamp(
    value: &str,
    format: &TimestampFormat,
    log: &ProvenanceLog,
) -> String {
    log.building_block(FORMATTED_TEXT_TO_TIMESTAMP);
    let sql = "t";
    let pattern = &format.pattern;
    let field = |wanted: TimestampField| {
        format
            .fields
            .iter()
            .position(|field| *field == wanted)
            .map(|index| format!("regexp_extract({sql}, '{pattern}', {})", index + 1))
    };
    let number = |wanted: TimestampField| {
        field(wanted).map_or_else(|| "0".to_owned(), |text| format!("CAST({text} AS INTEGER)"))
    };
    let hour = match field(TimestampField::Meridiem) {
        Some(meridiem) => format!(
            "(CASE WHEN {hour} NOT BETWEEN 1 AND 12 THEN NULL \
             ELSE {hour} % 12 + CASE WHEN upper({meridiem}) = 'PM' THEN 12 ELSE 0 END END)",
            hour = number(TimestampField::Hour12)
        ),
        None => number(TimestampField::Hour24),
    };
    let (minute, second) = (
        number(TimestampField::Minute),
        number(TimestampField::Second),
    );
    let spelling = raised(&format!(
        "{DATE_TEXT_SPELLING}: text not spelled in the format, which no recording shows Snowflake \
         reading or rejecting"
    ));
    let body = format!(
        "(CASE WHEN {sql} IS NULL OR {sql} = '' THEN NULL \
         WHEN regexp_full_match({sql}, '{pattern}') THEN \
         CASE WHEN {year} > 0 AND {hour} <= 23 AND {minute} <= 59 AND {second} <= 59 THEN \
         CAST(try(make_timestamp({year}, {month}, {day}, {hour}, {minute}, {second})) AS TIMESTAMP_NS) END \
         ELSE CAST({spelling} AS TIMESTAMP_NS) END)",
        year = number(TimestampField::Year),
        month = number(TimestampField::Month),
        day = number(TimestampField::Day),
    );
    bound(value, sql, &body)
}
