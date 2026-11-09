//! Number helpers: a NUMBER's largest value, the CASE reading text as one, and widened literals.

use crate::blocks::_helpers::text::space_trimmed;
use crate::blocks::constants::{
    BARE_EXPONENT_PATTERN, DECIMAL_TEXT_PATTERN, MANTISSA_FREE_PATTERN, MAX_DECIMAL_PRECISION,
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

/// The digits of `sql`, a `CAST(<digits> AS DECIMAL(p,s))` whose values `(precision, scale)` holds.
pub(crate) fn widened_literal(sql: &str, precision: u8, scale: u8) -> Option<&str> {
    let (digits, held) = sql
        .strip_prefix("CAST(")?
        .strip_suffix("))")?
        .split_once(" AS DECIMAL(")?;
    let (held_precision, held_scale) = held.split_once(',')?;
    let (held_precision, held_scale) = (small_number(held_precision)?, small_number(held_scale)?);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let plain = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    let fits = !whole.is_empty()
        && plain(whole)
        && plain(fraction)
        && whole.len() + fraction.len() <= usize::from(MAX_DECIMAL_PRECISION)
        && held_scale <= held_precision
        && whole.trim_start_matches('0').len() <= usize::from(held_precision - held_scale)
        && fraction.len() <= usize::from(held_scale)
        && held_scale <= scale
        && held_precision - held_scale <= precision.saturating_sub(scale);
    fits.then_some(digits)
}

/// The value of `digits`, at least one decimal digit and at most 255.
fn small_number(digits: &str) -> Option<u8> {
    if digits.is_empty() {
        return None;
    }
    digits.bytes().try_fold(0_u8, |value, byte| match byte {
        b'0'..=b'9' => value.checked_mul(10)?.checked_add(byte - b'0'),
        _ => None,
    })
}
