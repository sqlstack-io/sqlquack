//! `engine_release`: the year and release an engine version names.

use crate::formats::_helpers::dialect;

/// The `(year, release)` of an engine version such as `2026.45`, or none when it is not one.
pub fn engine_release(version: &str) -> Option<(u32, u32)> {
    dialect::engine_release(version)
}
