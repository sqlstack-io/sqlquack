//! `strftime_pattern`: a date or timestamp format as a strftime pattern, or None.

use crate::text_formats::_helpers::adjacency::{
    bare_fraction, field_at, field_recorded, separator_at,
};
use crate::text_formats::constants::BARE_FRACTION;

/// The strftime pattern of a format of known fields between separators, or None.
pub(crate) fn strftime_pattern(format: &str, precision: Option<u8>) -> Option<String> {
    let with_time = precision.is_some();
    let upper = format.to_ascii_uppercase();
    let mut rest = upper.as_str();
    let mut pattern = String::new();
    let mut last_letter: Option<char> = None;
    let mut previous: Option<&str> = None;
    while !rest.is_empty() {
        let written = &format[format.len() - rest.len()..];
        if let Some(after) = rest.strip_prefix(BARE_FRACTION)
            && !after.starts_with(|next: char| next.is_ascii_digit())
        {
            pattern.push_str(bare_fraction(previous, precision)?);
            rest = after;
            last_letter = None;
            previous = Some(BARE_FRACTION);
            continue;
        }
        match field_at(rest, with_time) {
            Some((token, replacement, _)) => {
                if !field_recorded(token, last_letter, previous, written) {
                    return None;
                }
                pattern.push_str(replacement);
                rest = &rest[token.len()..];
                last_letter = token.chars().last().filter(char::is_ascii_alphabetic);
                previous = Some(token);
            }
            None => {
                last_letter = None;
                previous = None;
                let separator = separator_at(rest)?;
                pattern.push(separator);
                rest = &rest[separator.len_utf8()..];
            }
        }
    }
    Some(pattern)
}
