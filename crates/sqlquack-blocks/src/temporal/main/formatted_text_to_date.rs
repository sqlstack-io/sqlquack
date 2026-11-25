//! `formatted_text_to_date`: TO_DATE and TRY_TO_DATE of text in one literal format, read exactly.

use sqlquack_diagnostics::failures::constants::DATE_FORMAT_MISMATCH;
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    DATE_TEXT_SPELLING, FORMATTED_TEXT_TO_DATE, TWO_DIGIT_CENTURY_START, UNPARSED_DATE_MESSAGE,
};
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::operands::once;
use crate::temporal::_helpers::text::{bound, digitless};
use crate::temporal::constants::FORMAT_GROUPS;
use crate::temporal::models::DateFormat;

/// The date `sql` spells exactly in `format`; impossible dates are NULL when `trying`, else raise.
pub fn formatted_text_to_date(
    value: &str,
    (format, trying): (&DateFormat, bool),
    log: &ProvenanceLog,
) -> String {
    log.building_block(FORMATTED_TEXT_TO_DATE);
    once(value, "formatted_date", |sql| {
        let pattern = format.pattern;
        let plain = converted(sql, (format, trying), &|group| {
            format!("regexp_extract({sql}, '{pattern}', {group})")
        });
        let (year, month, day) = format.groups;
        let names: Vec<String> = (1..=year.max(month).max(day))
            .map(|group| format!("'g{group}'"))
            .collect();
        let grouped = bound(
            &format!("regexp_extract({sql}, '{pattern}', [{}])", names.join(", ")),
            FORMAT_GROUPS,
            &converted(sql, (format, trying), &|group| {
                format!("{FORMAT_GROUPS}['g{group}']")
            }),
        );
        match grouped.len() < plain.len() {
            true => grouped,
            false => plain,
        }
    })
}

/// The conversion of the plain operand `sql`, each group's text as `extract` writes it.
fn converted(
    sql: &str,
    (format, trying): (&DateFormat, bool),
    extract: &dyn Fn(u8) -> String,
) -> String {
    let pattern = format.pattern;
    let field = |group: u8| format!("TRY_CAST({} AS INTEGER)", extract(group));
    let (year_group, month_group, day_group) = format.groups;
    let year = match format.two_digit_year {
        true => {
            let short = field(year_group);
            let century = TWO_DIGIT_CENTURY_START / 100 * 100;
            let pivot = TWO_DIGIT_CENTURY_START % 100;
            format!(
                "(CASE WHEN {short} < {pivot} THEN {} + {short} ELSE {century} + {short} END)",
                century + 100
            )
        }
        false => field(year_group),
    };
    let spelling = raised(&format!(
        "{DATE_TEXT_SPELLING}: text not spelled exactly in the format, which no recording shows \
         Snowflake reading or rejecting"
    ));
    let month = match format.named_month {
        true => format!(
            "list_position(['jan', 'feb', 'mar', 'apr', 'may', 'jun', 'jul', 'aug', 'sep', 'oct', \
             'nov', 'dec'], left(lower({}), 3))",
            extract(month_group)
        ),
        false => field(month_group),
    };
    let rejected = match (trying, format.null_pattern) {
        (true, Some(rejected)) => format!(
            "WHEN NOT regexp_full_match({sql}, '{pattern}') \
             AND regexp_full_match({sql}, '{rejected}') THEN NULL "
        ),
        _ => String::new(),
    };
    let (empty, impossible) = match trying {
        true => (
            format!("{sql} = '' OR {}", digitless(sql)),
            "NULL".to_owned(),
        ),
        false => (
            "false".to_owned(),
            raised(&tagged_message(DATE_FORMAT_MISMATCH, UNPARSED_DATE_MESSAGE)),
        ),
    };
    let foreign = match format.named_month {
        true => {
            format!("WHEN regexp_matches({sql}, '[^\\x00-\\x7F]') THEN {spelling} ")
        }
        false => String::new(),
    };
    format!(
        "(CASE WHEN {sql} IS NULL OR {empty} THEN NULL {foreign}\
         WHEN regexp_full_match({sql}, '{pattern}') AND {year} > 0 THEN \
         coalesce(try(make_date({year}, {month}, {})), {impossible}) \
         {rejected}ELSE {spelling} END)",
        field(day_group)
    )
}
