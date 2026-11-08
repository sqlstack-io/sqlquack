//! Exact decimal division's pieces: the short and long quotients, null checks and scaling.

use sqlquack_core::failures::constants::DIVISION_BY_ZERO;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::_helpers::text::{once_bound, tagged_error};
use crate::blocks::constants::{INTEGER_LITERAL_DIGITS, MAX_DECIMAL_PRECISION, SHORT_FORM_DIGITS};
use crate::blocks::main::cast::cast;
use crate::raises::main::raised::raised;

/// A decimal operand: its SQL, precision and scale.
type Operand<'a> = (&'a str, u8, u8);

/// The CASE arms making the quotient NULL, and raising for a zero divisor unless it is a constant.
pub(crate) fn null_checks(left_sql: &str, right_sql: &str, constant: bool) -> String {
    match constant {
        true => format!("WHEN {left_sql} IS NULL THEN NULL"),
        false => format!(
            "WHEN {left_sql} IS NULL OR {right_sql} IS NULL THEN NULL WHEN {right_sql} = 0 THEN {}",
            tagged_error(DIVISION_BY_ZERO, "Division by zero")
        ),
    }
}

/// The integer quotient `rounded` at the result scale, as the result type.
pub(crate) fn scaled(
    rounded: &str,
    (result, result_scale): (SnowflakeType, u8),
    log: &ProvenanceLog,
) -> Result<String, String> {
    let unit = match result_scale {
        0 => "1".to_owned(),
        scale => format!("0.{}1", "0".repeat(usize::from(scale) - 1)),
    };
    cast(
        &format!("CAST({rounded} AS DECIMAL(38,0)) * CAST('{unit}' AS DECIMAL(38,{result_scale}))"),
        result,
        log,
    )
}

/// The quotient in units of the result scale when the scaled dividend fits 128 bits; else `None`.
pub(crate) fn short_quotient(
    (left, right): (Operand<'_>, Operand<'_>),
    (result_scale, truncated): (u8, bool),
    constant: Option<u128>,
) -> Option<String> {
    let (_, left_precision, left_scale) = left;
    let (_, right_precision, right_scale) = right;
    let shift = (result_scale + right_scale).checked_sub(left_scale)?;
    let digits = usize::from(left_precision) + usize::from(shift);
    let room = match constant {
        Some(_) => SHORT_FORM_DIGITS,
        None => usize::from(MAX_DECIMAL_PRECISION),
    };
    let fits = fits_unscaled(left_precision, left_scale)
        && fits_unscaled(right_precision, right_scale)
        && digits <= room;
    let doubles = digits <= SHORT_FORM_DIGITS && usize::from(right_precision) <= SHORT_FORM_DIGITS;
    fits.then(|| {
        once_bound(
            &unscaled(left, left_scale + shift),
            "quotient_dividend",
            |n| match constant {
                Some(divisor) => constant_quotient(n, divisor, truncated),
                None => once_bound(&unscaled(right, right_scale), "quotient_divisor", |d| {
                    column_quotient(n, d, (truncated, doubles))
                }),
            },
        )
    })
}

/// The quotient over HUGEINT pieces, and the CASE arm raising where it needs more than 128 bits.
pub(crate) fn long_quotient(
    (left, right): (Operand<'_>, Operand<'_>),
    (result_scale, truncated): (u8, bool),
    constant: Option<u128>,
) -> Result<(String, Option<String>), String> {
    let (left_sql, _, left_scale) = left;
    let (right_sql, right_precision, right_scale) = right;
    let shift = result_scale + right_scale - left_scale.min(result_scale + right_scale);
    if shift >= MAX_DECIMAL_PRECISION {
        return Err(format!(
            "a scale shift of {shift} leaves no room in DuckDB's 128-bit integers"
        ));
    }
    let dividend = long_unscaled(left_sql, left_scale);
    let divisor = match constant {
        Some(divisor) => integer(divisor),
        None => long_unscaled(right_sql, right_scale),
    };
    let power = format!("CAST('1{}' AS HUGEINT)", "0".repeat(usize::from(shift)));
    let remainder = format!("(({dividend} % {divisor}) * {power})");
    let rounding = match truncated {
        true => "0".to_owned(),
        false => format!(
            "(CASE WHEN abs({remainder} % {divisor}) >= abs({divisor}) - abs({remainder} % {divisor}) \
             THEN sign({remainder}) * sign({divisor}) ELSE 0 END)"
        ),
    };
    let guard = (u32::from(right_precision) + u32::from(shift) > u32::from(MAX_DECIMAL_PRECISION))
        .then(|| {
            format!(
                " WHEN abs({divisor}) >= CAST('1{}' AS HUGEINT) THEN {}",
                "0".repeat(usize::from(MAX_DECIMAL_PRECISION - shift)),
                raised("numeric_range: the exact quotient needs more than 128 bits")
            )
        });
    Ok((
        format!("(({dividend} // {divisor}) * {power} + ({remainder} // {divisor}) + {rounding})"),
        guard,
    ))
}

/// The integer a plain number literal, such as `2.50`, is at `scale` places, if short enough.
pub(crate) fn unscaled_literal(text: &str, scale: u8) -> Option<u128> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let scale = usize::from(scale);
    if !whole
        .bytes()
        .chain(fraction.bytes())
        .all(|byte| byte.is_ascii_digit())
        || fraction.len() > scale
        || whole.len() + scale > SHORT_FORM_DIGITS
    {
        return None;
    }
    let padded = format!("{whole}{fraction}{}", "0".repeat(scale - fraction.len()));
    Some(
        padded
            .bytes()
            .fold(0, |value, digit| value * 10 + u128::from(digit - b'0')),
    )
}

/// The scaled dividend `n` over a known non-zero divisor, rounded half away from zero or truncated.
fn constant_quotient(n: &str, divisor: u128, truncated: bool) -> String {
    match truncated {
        true => format!("({n} // {})", integer(divisor)),
        false => format!(
            "(sign({n}) * ((2 * abs({n}) + {}) // {}))",
            integer(divisor),
            integer(2 * divisor)
        ),
    }
}

/// The scaled dividend `n` over the divisor `d`, doubling to round when both leave a digit spare.
fn column_quotient(n: &str, d: &str, (truncated, doubles): (bool, bool)) -> String {
    match (truncated, doubles) {
        (true, _) => format!("({n} // {d})"),
        (false, true) => {
            format!("(sign({n}) * sign({d}) * ((2 * abs({n}) + abs({d})) // (2 * abs({d}))))")
        }
        (false, false) => format!(
            "({n} // {d} + (CASE WHEN abs({n} % {d}) >= abs({d}) - abs({n} % {d}) \
             THEN sign({n}) * sign({d}) ELSE 0 END))"
        ),
    }
}

/// The decimal `sql` of `scale` places as an unscaled HUGEINT, from its digits past 19 places.
fn long_unscaled(sql: &str, scale: u8) -> String {
    let decimal = format!("CAST({sql} AS DECIMAL(38,{scale}))");
    if 2 * usize::from(scale) > usize::from(MAX_DECIMAL_PRECISION) {
        return format!("CAST(replace(CAST({decimal} AS VARCHAR), '.', '') AS HUGEINT)");
    }
    let power = "0".repeat(usize::from(scale));
    format!(
        "(CAST(trunc({decimal}) AS HUGEINT) * CAST('1{power}' AS HUGEINT) + \
         CAST(({decimal} - trunc({decimal})) * CAST('1{power}' AS DECIMAL(38,0)) AS HUGEINT))"
    )
}

/// Whether a NUMBER(precision, scale) times 10 to its scale fits DuckDB's DECIMAL(38, scale).
fn fits_unscaled(precision: u8, scale: u8) -> bool {
    usize::from(precision) + usize::from(scale) <= usize::from(MAX_DECIMAL_PRECISION)
}

/// The decimal `sql` times 10 to the `places` as an integer: one DECIMAL product when it fits.
fn unscaled((sql, precision, scale): Operand<'_>, places: u8) -> String {
    let at_scale = match scale {
        0 => format!("CAST({sql} AS HUGEINT)"),
        _ if usize::from(precision) + usize::from(places) <= usize::from(MAX_DECIMAL_PRECISION) => {
            return format!(
                "CAST(CAST({sql} AS DECIMAL(38,{scale})) * {} AS HUGEINT)",
                power(places)
            );
        }
        _ => format!(
            "CAST(CAST({sql} AS DECIMAL(38,{scale})) * {} AS HUGEINT)",
            power(scale)
        ),
    };
    match places - scale {
        0 => at_scale,
        more => format!("({at_scale} * {})", power(more)),
    }
}

/// 10 to the `places` as SQL: an integer literal while DuckDB reads one, else a HUGEINT.
fn power(places: u8) -> String {
    let digits = "0".repeat(usize::from(places));
    match usize::from(places) <= INTEGER_LITERAL_DIGITS {
        true => format!("1{digits}"),
        false => format!("CAST('1{digits}' AS HUGEINT)"),
    }
}

/// An integer as SQL: a literal while DuckDB reads it as one, else a HUGEINT cast of its text.
fn integer(value: u128) -> String {
    match value <= u128::from(i64::MAX.unsigned_abs()) {
        true => value.to_string(),
        false => format!("CAST('{value}' AS HUGEINT)"),
    }
}
