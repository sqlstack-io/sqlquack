//! Number text helpers: the largest value of a NUMBER and the CASE reading text as one.

use crate::blocks::_helpers::text::space_trimmed;
use crate::blocks::constants::{
    BARE_EXPONENT_PATTERN, DECIMAL_TEXT_PATTERN, MANTISSA_FREE_PATTERN,
};
use crate::blocks::models::NumberTarget;

/// The largest NUMBER(`precision`, `scale`) as text, such as `9999.99`.
pub(crate) fn largest_number(precision: u8, scale: u8) -> String {
    let whole = "9".repeat(usize::from(precision - scale));
    match scale {
        0 => whole,
        _ => format!("{whole}.{}", "9".repeat(usize::from(scale))),
    }
}

/// The CASE reading the text `sql` as `target`, accepting only spellings Snowflake reads.
pub(crate) fn read_number_as(sql: &str, target: &NumberTarget) -> String {
    let wrapped =
        |(before, after): &(String, String), value: &str| format!("{before}{value}{after}");
    let trimmed = space_trimmed(sql);
    let parsed = wrapped(&target.try_cast, &trimmed);
    let overflow = &target.overflow;
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL \
         WHEN regexp_matches({trimmed}, '[eE]') AND abs({parsed}) > {largest} THEN {overflow} \
         WHEN regexp_full_match({trimmed}, '{DECIMAL_TEXT_PATTERN}') \
         THEN coalesce({parsed}, {overflow}) \
         WHEN regexp_full_match({trimmed}, '{BARE_EXPONENT_PATTERN}') \
         THEN coalesce({}, {overflow}) \
         WHEN regexp_full_match({trimmed}, '{MANTISSA_FREE_PATTERN}') THEN {} \
         ELSE {} END)",
        wrapped(
            &target.try_cast,
            &format!("left({trimmed}, length({trimmed}) - 1)")
        ),
        wrapped(&target.cast, "0"),
        target.unrecognised,
        largest = target.largest,
    )
}
