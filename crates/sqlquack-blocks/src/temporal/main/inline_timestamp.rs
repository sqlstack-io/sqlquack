//! `inline_timestamp`: `text_to_timestamp` written out, for its helper macros' bodies and VARIANT reads.

use sqlquack_diagnostics::failures::constants::{DATE_NOT_RECOGNIZED, TIMESTAMP_NOT_RECOGNIZED};

use crate::blocks::constants::{
    DATE_TEXT_SPELLING, DAY_MONTH_NAME_DATE_PATTERN, EPOCH_SCALE_LIMITS, EPOCH_TEXT_PATTERN,
    ISO_TIMESTAMP_PATTERN, MONTH_ABBREVIATIONS, MONTH_FIRST_DATE_PATTERN,
    MONTH_FIRST_TIMESTAMP_PATTERN, NULL_DATE_TEXT_PATTERN, NULL_TIMESTAMP_TEXT_PATTERN,
    RFC_UTC_TIMESTAMP_PATTERN, SECONDS_PER_DAY, TIMESTAMP_RANGE, UNPADDED_TIMESTAMP_PATTERN,
};
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::operands::once;
use crate::temporal::_helpers::text::{checked, digitless, parse_failure, whitespace_trimmed};

/// `text_to_timestamp` written out, as a helper macro's body and a VARIANT's string read it.
pub fn inline_timestamp(value: &str, (on_failure, date_only): (TextFailure, bool)) -> String {
    once(value, "timestamp_text", |sql| {
        let trimmed = whitespace_trimmed(sql);
        let failure = parse_failure(
            on_failure,
            (TIMESTAMP_NOT_RECOGNIZED, "Timestamp is not recognized"),
        );
        let date = format!("regexp_extract({trimmed}, '^{ISO_TIMESTAMP_PATTERN}$', 1)");
        if date_only {
            return text_to_date(sql, (&trimmed, on_failure));
        }
        let [seconds, millis, micros] = EPOCH_SCALE_LIMITS;
        let number = format!("CAST({trimmed} AS BIGINT)");
        let group =
            |index: u8| format!("regexp_extract({trimmed}, '^{ISO_TIMESTAMP_PATTERN}$', {index})");
        let clock = format!(
            "{date} || ' ' || coalesce(nullif({}, ''), '00:00') || coalesce(nullif({}, ''), ':00') \
         || coalesce('.' || left(nullif({}, ''), 9), '')",
            group(2),
            group(4),
            group(5)
        );
        let parsed =
            |format: &str| format!("CAST(try_strptime({trimmed}, '{format}') AS TIMESTAMP_NS)");
        let rfc = format!(
            "try_strptime(left({trimmed}, length({trimmed}) - 6), '%a, %d %b %Y %H:%M:%S')"
        );
        let unpadded = format!(
            "CASE WHEN contains({trimmed}, ' ') THEN {} ELSE {} END",
            parsed("%Y-%m-%d %H:%M:%S"),
            parsed("%Y-%m-%d")
        );
        let converted = format!(
            "(CASE WHEN {sql} IS NULL THEN NULL \
         WHEN regexp_full_match({trimmed}, '{EPOCH_TEXT_PATTERN}') THEN make_timestamp_ns( \
         CASE WHEN {number} < {seconds} THEN {number} * 1000000000 \
         WHEN {number} < {millis} THEN {number} * 1000000 \
         WHEN {number} < {micros} THEN {number} * 1000 ELSE {number} END) \
         WHEN regexp_full_match({trimmed}, '{ISO_TIMESTAMP_PATTERN}') \
         THEN coalesce(TRY_CAST({clock} AS TIMESTAMP_NS), CASE WHEN TRY_CAST({clock} AS TIMESTAMP) \
         IS NOT NULL THEN CAST({beyond} AS TIMESTAMP_NS) END) \
         WHEN regexp_full_match({trimmed}, '{UNPADDED_TIMESTAMP_PATTERN}') \
         THEN {unpadded} \
         WHEN regexp_full_match({trimmed}, '{MONTH_FIRST_TIMESTAMP_PATTERN}') \
         THEN {month_first} \
         WHEN regexp_full_match({trimmed}, '{RFC_UTC_TIMESTAMP_PATTERN}') \
         AND strftime({rfc}, '%a') = left({trimmed}, 3) THEN CAST({rfc} AS TIMESTAMP_NS) \
         ELSE CAST({unrecorded} AS TIMESTAMP_NS) END)",
            month_first = parsed("%m/%d/%Y %H:%M:%S"),
            unrecorded = match on_failure {
                TextFailure::Null => format!(
                    "CASE WHEN {trimmed} = '' OR {} OR regexp_full_match({trimmed}, \
                 '{NULL_TIMESTAMP_TEXT_PATTERN}') THEN NULL ELSE {} END",
                    digitless(&trimmed),
                    unrecorded_spelling(TextFailure::Text, &failure)
                ),
                TextFailure::Text | TextFailure::Variant =>
                    unrecorded_spelling(on_failure, &failure),
            },
            beyond = raised(&format!(
                "{TIMESTAMP_RANGE}: a timestamp Snowflake holds outside the nanosecond range of DuckDB"
            )),
        );
        checked(sql, &converted, &failure)
    })
}

/// Text to DATE in the recorded AUTO spellings and epoch integer text, without a timestamp's range.
fn text_to_date(sql: &str, (trimmed, on_failure): (&str, TextFailure)) -> String {
    let impossible = parse_failure(on_failure, (DATE_NOT_RECOGNIZED, "Date is not recognized"));
    let failure = match on_failure {
        TextFailure::Null => format!(
            "CASE WHEN {trimmed} = '' OR {} OR regexp_full_match({trimmed}, \
             '{NULL_DATE_TEXT_PATTERN}') THEN NULL ELSE {} END",
            digitless(trimmed),
            raised(&format!(
                "{DATE_TEXT_SPELLING}: text in a spelling no recording shows Snowflake reading or \
                 rejecting"
            ))
        ),
        TextFailure::Text => unrecorded_spelling(on_failure, &impossible),
        TextFailure::Variant => impossible.clone(),
    };
    let named = |group: u8| {
        format!("regexp_extract({trimmed}, '^{DAY_MONTH_NAME_DATE_PATTERN}$', {group})")
    };
    let (convert, parse_name) = ("TRY_CAST", "try_strptime");
    let date = format!("regexp_extract({trimmed}, '^{ISO_TIMESTAMP_PATTERN}$', 1)");
    let [seconds, millis, micros] = EPOCH_SCALE_LIMITS;
    let number = format!("CAST({trimmed} AS BIGINT)");
    let rfc =
        format!("{parse_name}(left({trimmed}, length({trimmed}) - 6), '%a, %d %b %Y %H:%M:%S')");
    let months = MONTH_ABBREVIATIONS
        .iter()
        .map(|month| format!("'{month}'"))
        .collect::<Vec<String>>()
        .join(", ");
    let converted = format!(
        "(CASE WHEN {sql} IS NULL THEN NULL \
         WHEN regexp_full_match({trimmed}, '{ISO_TIMESTAMP_PATTERN}') THEN {convert}({date} AS DATE) \
         WHEN regexp_full_match({trimmed}, '{MONTH_FIRST_DATE_PATTERN}') THEN {convert}( \
         substr({trimmed}, 7, 4) || '-' || substr({trimmed}, 1, 2) || '-' || substr({trimmed}, 4, 2) AS DATE) \
         WHEN regexp_full_match({trimmed}, '{DAY_MONTH_NAME_DATE_PATTERN}') THEN {convert}( \
         lpad({}, 4, '0') || '-' || lpad(CAST(list_position([{months}], upper({})) AS VARCHAR), 2, '0') \
         || '-' || lpad({}, 2, '0') AS DATE) \
         WHEN regexp_full_match({trimmed}, '{UNPADDED_TIMESTAMP_PATTERN}') THEN CAST(CASE WHEN \
         contains({trimmed}, ' ') THEN {parse_name}({trimmed}, '%Y-%m-%d %H:%M:%S') \
         ELSE {parse_name}({trimmed}, '%Y-%m-%d') END AS DATE) \
         WHEN regexp_full_match({trimmed}, '{RFC_UTC_TIMESTAMP_PATTERN}') \
         AND strftime({rfc}, '%a') = left({trimmed}, 3) THEN CAST({rfc} AS DATE) \
         WHEN regexp_full_match({trimmed}, '{EPOCH_TEXT_PATTERN}') THEN DATE '1970-01-01' + CAST( \
         {number} // CASE WHEN {number} < {seconds} THEN {SECONDS_PER_DAY} \
         WHEN {number} < {millis} THEN {SECONDS_PER_DAY} * 1000 \
         WHEN {number} < {micros} THEN {SECONDS_PER_DAY} * 1000000 \
         ELSE {SECONDS_PER_DAY} * 1000000000 END AS INTEGER) \
         ELSE CAST({failure} AS DATE) END)",
        named(3),
        named(2),
        named(1)
    );
    checked(sql, &converted, &impossible)
}

/// Non-TRY text in an unrecorded spelling raises date_text_spelling; others keep `failure`.
fn unrecorded_spelling(on_failure: TextFailure, failure: &str) -> String {
    match on_failure {
        TextFailure::Text => raised(&format!(
            "{DATE_TEXT_SPELLING}: text in a spelling no recording shows Snowflake reading or \
             rejecting"
        )),
        TextFailure::Null | TextFailure::Variant => failure.to_owned(),
    }
}
