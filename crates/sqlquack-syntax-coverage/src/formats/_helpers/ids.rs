//! The shapes of ids and timestamps.

use crate::formats::constants::{
    CASE_ID_LIMIT, DIGIT_PLACEHOLDER, FORM_ID_MIN_PARTS, FORM_PREFIX, OBLIGATION_DIGITS,
    OBLIGATION_MARKER, SOURCE_PREFIX, UTC_SHAPE, UTC_SUFFIXES,
};

/// A lower-case name segment: letters, digits and inner hyphens or underscores.
pub(crate) fn is_segment(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
        && text
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && text
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

/// Whether `text` is an obligation number such as `o001` or `o1000`: three digits, or more without
/// a leading zero, so each number has one spelling.
pub(crate) fn is_obligation_number(text: &str) -> bool {
    text.strip_prefix(OBLIGATION_MARKER).is_some_and(|digits| {
        digits.bytes().all(|byte| byte.is_ascii_digit())
            && (digits.len() == OBLIGATION_DIGITS
                || (digits.len() > OBLIGATION_DIGITS && !digits.starts_with('0')))
    })
}

/// `sf.<family>.<page-slug>[.<variant>…]`; no segment may read as an obligation number.
pub(crate) fn is_form_id(id: &str, family: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() >= FORM_ID_MIN_PARTS
        && parts[0] == FORM_PREFIX
        && parts[1] == family
        && parts[2..]
            .iter()
            .all(|part| is_segment(part) && !is_obligation_number(part))
}

/// `doc.<page-slug>[.<sub-page>…]`.
pub(crate) fn is_source_id(id: &str) -> bool {
    id.strip_prefix(SOURCE_PREFIX)
        .and_then(|rest| rest.strip_prefix('.'))
        .is_some_and(|rest| rest.split('.').all(is_segment))
}

/// A non-empty case id without whitespace or control characters.
pub(crate) fn is_case_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().count() <= CASE_ID_LIMIT
        && !id
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

/// `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn is_utc(text: &str) -> bool {
    text.len() == UTC_SHAPE.len()
        && text.bytes().zip(UTC_SHAPE.bytes()).all(|(byte, shape)| {
            if shape == DIGIT_PLACEHOLDER {
                byte.is_ascii_digit()
            } else {
                byte == shape
            }
        })
}

/// RFC 3339 in UTC: `YYYY-MM-DDTHH:MM:SS`, an optional fraction, then `Z` or `+00:00`.
pub(crate) fn is_rfc3339_utc(text: &str) -> bool {
    let Some(rest) = UTC_SUFFIXES
        .iter()
        .find_map(|suffix| text.strip_suffix(suffix))
    else {
        return false;
    };
    let Some((seconds, fraction)) = rest.split_at_checked(rest.len().min(UTC_SHAPE.len() - 1))
    else {
        return false;
    };
    let fraction_valid: bool = match fraction.strip_prefix('.') {
        Some(digits) => !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()),
        None => fraction.is_empty(),
    };
    fraction_valid && is_utc(&format!("{seconds}Z"))
}
