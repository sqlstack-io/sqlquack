//! `literal_digits`: a numeric literal's value as plain decimal digits.

use crate::typing::constants::{MAX_EXPONENT_DIGITS, MAX_POINT_SHIFT};

/// The literal's value without an exponent, as `digits[.digits]`, or why it is not a literal.
pub fn literal_digits(text: &str) -> Result<String, String> {
    let refused = || format!("`{text}` is not a numeric literal");
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent),
        None => (text, "0"),
    };
    let (negative, magnitude) = match exponent.strip_prefix('-') {
        Some(magnitude) => (true, magnitude),
        None => (false, exponent.strip_prefix('+').unwrap_or(exponent)),
    };
    if magnitude.is_empty()
        || magnitude.len() > MAX_EXPONENT_DIGITS
        || !magnitude.bytes().all(|byte| byte.is_ascii_digit())
        || mantissa.is_empty()
        || !mantissa
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Err(refused());
    }
    let shift = magnitude
        .bytes()
        .fold(0_usize, |total, byte| total * 10 + usize::from(byte - b'0'));
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if shift > MAX_POINT_SHIFT
        || fraction.contains('.')
        || (whole.is_empty() && fraction.is_empty())
    {
        return Err(refused());
    }
    let digits = format!("{whole}{fraction}");
    let (whole, fraction) = match negative {
        false => {
            let point = whole.len() + shift;
            let padded = format!("{digits}{}", "0".repeat(point.saturating_sub(digits.len())));
            let (whole, fraction) = padded.split_at(point);
            (whole.to_owned(), fraction.to_owned())
        }
        true if shift >= whole.len() => (
            String::new(),
            format!("{}{digits}", "0".repeat(shift - whole.len())),
        ),
        true => {
            let (whole, fraction) = digits.split_at(whole.len() - shift);
            (whole.to_owned(), fraction.to_owned())
        }
    };
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    Ok(match fraction.is_empty() {
        true => whole.to_owned(),
        false => format!("{whole}.{fraction}"),
    })
}
