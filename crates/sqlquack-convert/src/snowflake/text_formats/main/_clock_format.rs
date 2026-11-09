//! `clock_format`: whether a TIME format names only recorded clock fields.

use crate::snowflake::text_formats::constants::CLOCK_FORMAT_FIELDS;

/// Whether a format names only clock fields, HH24, MI, SS and up to six fraction digits.
pub(in crate::snowflake) fn clock_format(written: &str) -> bool {
    let mut rest = written;
    while !rest.is_empty() {
        let Some(field) = CLOCK_FORMAT_FIELDS
            .iter()
            .find(|field| rest.starts_with(**field))
        else {
            return false;
        };
        rest = &rest[field.len()..];
    }
    true
}
