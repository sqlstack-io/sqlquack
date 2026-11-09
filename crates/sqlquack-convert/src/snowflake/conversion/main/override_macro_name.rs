//! `override_macro_name`: the name converted SQL calls a user override's macro by.

use crate::snowflake::conversion::constants::OVERRIDE_MACRO_PREFIX;

/// The name converted SQL calls a user override by.
pub fn override_macro_name(name: &str) -> String {
    format!("{OVERRIDE_MACRO_PREFIX}{name}")
}
