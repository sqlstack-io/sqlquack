//! `text_to_time`: text read as a TIME, in Snowflake's recorded spellings.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::DATE_TEXT_SPELLING;
use crate::blocks::main::string_literal::string_literal;
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::constants::NANOS_PER_MINUTE;
use crate::times::_helpers::failures::time_failure;
use crate::times::_helpers::nanos::{time_of_nanos, truncated_nanos};
use crate::times::constants::{
    AUTO_TIME_PATTERN, AUTO_TIME_REJECTED, COMPACT_HOUR_MINUTE_PATTERN,
    COMPACT_HOUR_MINUTE_REJECTED, HOUR_MINUTE_PATTERN, HOUR_MINUTE_REJECTED, MICROSECOND_PRECISION,
    NANOS_PER_SECOND, TEXT_TO_TIME, TIME_GROUPS, TIME_NANOS, TIME_SUBMICRO,
};
use crate::times::models::TimeFormat;

/// The TIME_NS the text spells, cut to `precision`; unrecorded spellings raise divergences.
pub fn text_to_time(
    sql: &str,
    (format, precision): (TimeFormat, u8),
    failure: TextFailure,
    log: &ProvenanceLog,
) -> String {
    log.building_block(TEXT_TO_TIME);
    let text = "x";
    let (pattern, rejected) = match format {
        TimeFormat::Auto => (AUTO_TIME_PATTERN, AUTO_TIME_REJECTED),
        TimeFormat::HourMinute => (HOUR_MINUTE_PATTERN, HOUR_MINUTE_REJECTED),
        TimeFormat::CompactHourMinute => {
            (COMPACT_HOUR_MINUTE_PATTERN, COMPACT_HOUR_MINUTE_REJECTED)
        }
    };
    let pattern = string_literal(pattern, log);
    let names = match format {
        TimeFormat::Auto => "['h', 'm', 'r', 's', 'q', 'f']",
        TimeFormat::HourMinute | TimeFormat::CompactHourMinute => "['h', 'm']",
    };
    let number =
        |name: &str| format!("CAST(coalesce(nullif({TIME_GROUPS}['{name}'], ''), '0') AS BIGINT)");
    let (hour, minute) = (number("h"), number("m"));
    let (second, fraction) = match format {
        TimeFormat::Auto => (
            number("s"),
            format!("CAST(rpad({TIME_GROUPS}['f'], 9, '0') AS BIGINT)"),
        ),
        _ => ("0".to_owned(), "0".to_owned()),
    };
    let nanos = format!(
        "({hour} * {} + {minute} * {NANOS_PER_MINUTE} + {second} * {NANOS_PER_SECOND} + {fraction})",
        NANOS_PER_MINUTE * 60
    );
    let failed = format!(
        "CAST({} AS TIME_NS)",
        time_failure(failure, format != TimeFormat::Auto)
    );
    let unproven = format!(
        "CAST({} AS TIME_NS)",
        raised(&format!(
            "{DATE_TEXT_SPELLING}: text spelled as no recording shows Snowflake reading or \
             rejecting as a time"
        ))
    );
    let submicro = match format {
        TimeFormat::Auto if precision > MICROSECOND_PRECISION => format!(
            "WHEN {fraction} % 1000 <> 0 THEN CAST({} AS TIME_NS) ",
            raised(&format!(
                "{TIME_SUBMICRO}: a TIME with a sub-microsecond part, which recordings cannot show"
            ))
        ),
        _ => String::new(),
    };
    let recorded_rejections: Vec<String> = rejected
        .iter()
        .map(|rejected| {
            format!(
                " OR regexp_matches({text}, {})",
                string_literal(rejected, log)
            )
        })
        .collect();
    let body = format!(
        "(CASE WHEN {text} IS NULL THEN NULL \
         WHEN {TIME_GROUPS}['h'] <> '' THEN \
         CASE WHEN {hour} > 23 OR {minute} > 59 THEN {failed} \
         WHEN {second} > 59 THEN {unproven} {submicro}ELSE list_transform([{nanos}], \
         lambda {TIME_NANOS}: {})[1] END \
         WHEN trim({text}, ' ') = '' OR NOT regexp_matches({text}, '[0-9]'){} THEN {failed} \
         ELSE {unproven} END)",
        time_of_nanos(&truncated_nanos(TIME_NANOS, precision)),
        recorded_rejections.concat()
    );
    format!(
        "list_transform([{sql}], lambda {text}: list_transform([regexp_extract({text}, {pattern}, \
         {names})], lambda {TIME_GROUPS}: {body})[1])[1]"
    )
}
