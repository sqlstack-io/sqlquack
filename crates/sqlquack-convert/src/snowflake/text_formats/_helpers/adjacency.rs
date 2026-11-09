//! Which format fields may follow one another with no separator between.

use crate::snowflake::text_formats::constants::{
    BARE_FRACTION, BARE_FRACTION_PATTERNS, MONTH_NAME_FIELDS, MONTH_NAME_FOLLOWERS,
    TEMPORAL_TEXT_FIELDS, TEMPORAL_TEXT_SEPARATORS,
};

/// Whether `next` may follow the field `previous` with no separator between, as recordings show.
pub(crate) fn adjacency_recorded(previous: Option<&str>, next: &str) -> bool {
    match previous {
        None => true,
        Some(previous) => match MONTH_NAME_FOLLOWERS
            .iter()
            .find(|(name, _)| *name == previous)
        {
            Some((_, followers)) => followers.contains(&next),
            None => !MONTH_NAME_FIELDS.contains(&next),
        },
    }
}

/// What a bare FF writes after `previous` at `precision`, or None where no recording shows it.
pub(crate) fn bare_fraction(previous: Option<&str>, precision: Option<u8>) -> Option<&'static str> {
    if !adjacency_recorded(previous, BARE_FRACTION) {
        return None;
    }
    BARE_FRACTION_PATTERNS
        .iter()
        .find(|(digits, _)| Some(*digits) == precision)
        .map(|(_, pattern)| *pattern)
}

/// Whether `token` may stand here, after `previous`, written as recorded (month names upper case).
pub(crate) fn field_recorded(
    token: &str,
    last_letter: Option<char>,
    previous: Option<&str>,
    written: &str,
) -> bool {
    !(last_letter.is_some_and(|letter| token.starts_with(letter))
        || !adjacency_recorded(previous, token)
        || (MONTH_NAME_FIELDS.contains(&token) && !written.starts_with(token)))
}

/// The field `rest` starts with: its token, strftime spelling and whether it is a time field.
pub(crate) fn field_at(
    rest: &str,
    with_time: bool,
) -> Option<&'static (&'static str, &'static str, bool)> {
    TEMPORAL_TEXT_FIELDS
        .iter()
        .find(|(token, _, time)| rest.starts_with(token) && (with_time || !time))
}

/// The separator `rest` starts with, or None when it starts with no recorded separator.
pub(crate) fn separator_at(rest: &str) -> Option<char> {
    rest.chars()
        .next()
        .filter(|separator| TEMPORAL_TEXT_SEPARATORS.contains(*separator))
}
