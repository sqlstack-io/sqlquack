//! `formatted_text_to_timestamp`: TO_TIMESTAMP_NTZ and TRY_TO_TIMESTAMP_NTZ of text in one literal format.

use sqlquack_core::failures::constants::TIMESTAMP_FORMAT_MISMATCH;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    DATE_TEXT_SPELLING, FORMATTED_TEXT_TO_TIMESTAMP, TIMESTAMP_RANGE, TWO_DIGIT_CENTURY_START,
    UNPARSED_TIMESTAMP_MESSAGE, UNTRIMMED_SPACE,
};
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::text::{bound, digitless};
use crate::temporal::models::{TimestampField, TimestampFormat};

/// `value` in `format`: misspelled raises, but is NULL for the TRY_ forms of YY and MON formats,
/// as recorded; empty or impossible is NULL if `trying`, else raises.
pub fn formatted_text_to_timestamp(
    value: &str,
    (format, trying): (&TimestampFormat, bool),
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
    let year = match field(TimestampField::ShortYear) {
        Some(short) => {
            let century = TWO_DIGIT_CENTURY_START / 100 * 100;
            let pivot = TWO_DIGIT_CENTURY_START % 100;
            format!(
                "(CASE WHEN CAST({short} AS INTEGER) < {pivot} THEN {} ELSE {century} END \
                 + CAST({short} AS INTEGER))",
                century + 100
            )
        }
        None => number(TimestampField::Year),
    };
    let month = match field(TimestampField::MonthName) {
        Some(name) => format!(
            "list_position(['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', \
             'nov', 'dec'], left(lower({name}), 3))"
        ),
        None => number(TimestampField::Month),
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
    let (empty, impossible) = match trying {
        true => (
            format!("{sql} = '' OR {}", digitless(sql)),
            "NULL".to_owned(),
        ),
        false => (
            "false".to_owned(),
            format!(
                "CAST({} AS TIMESTAMP_NS)",
                raised(&tagged_message(
                    TIMESTAMP_FORMAT_MISMATCH,
                    UNPARSED_TIMESTAMP_MESSAGE
                ))
            ),
        ),
    };
    let strict = trying
        && format
            .fields
            .iter()
            .any(|field| matches!(field, TimestampField::ShortYear | TimestampField::MonthName));
    let misspelled = match strict {
        true => format!("CASE WHEN regexp_matches({sql}, '{UNTRIMMED_SPACE}') THEN {spelling} END"),
        false => spelling,
    };
    let rejected = match (trying, &format.rejected) {
        (true, Some(rejected)) => format!("WHEN regexp_full_match({sql}, '{rejected}') THEN NULL "),
        _ => String::new(),
    };
    let body = format!(
        "(CASE WHEN {sql} IS NULL OR {empty} THEN NULL WHEN {sql} = '' THEN {impossible} {rejected}\
         WHEN regexp_full_match({sql}, '{pattern}') THEN \
         coalesce(CASE WHEN {year} > 0 AND {hour} <= 23 AND {minute} <= 59 AND {second} <= 59 THEN \
         list_transform([try(make_timestamp({year}, {month}, {day}, {hour}, {minute}, {second}))], \
         lambda made: CASE WHEN made IS NULL THEN NULL ELSE coalesce(TRY_CAST(made AS TIMESTAMP_NS), \
         CAST({range} AS TIMESTAMP_NS)) END)[1] END, \
         {impossible}) \
         ELSE CAST({misspelled} AS TIMESTAMP_NS) END)",
        day = number(TimestampField::Day),
        range = raised(&format!(
            "{TIMESTAMP_RANGE}: a timestamp Snowflake holds outside the nanosecond range of DuckDB"
        )),
    );
    match strict {
        true => bound(&format!("trim({value}, ' ')"), sql, &body),
        false => bound(value, sql, &body),
    }
}
