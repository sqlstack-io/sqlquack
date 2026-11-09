//! `is_utc_timestamp`: the UTC timestamp format every coverage record uses.

use crate::formats::_helpers::ids::is_utc;

/// Whether `text` is a UTC timestamp such as `2026-11-04T10:00:00Z`.
pub fn is_utc_timestamp(text: &str) -> bool {
    is_utc(text)
}
