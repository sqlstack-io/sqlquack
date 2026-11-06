//! `recorded_zone`: whether Snowflake was recorded accepting a time zone name.

use crate::time_zones::constants::RECORDED_TIME_ZONES;

/// Whether Snowflake was recorded accepting `name`, which it matches exactly, case included.
pub fn recorded_zone(name: &str) -> bool {
    RECORDED_TIME_ZONES.contains(&name)
}
