//! `tagged_message`: the text of an error a building block raises.

use crate::failures::constants::ERROR_TAG_PREFIX;
use crate::failures::models::SnowflakeCode;

/// `sqlquack:<errno>: <message>`, which `duckdb_failure` reads back as that Snowflake error.
pub fn tagged_message(code: SnowflakeCode, message: &str) -> String {
    format!("{ERROR_TAG_PREFIX}{}: {message}", code.errno)
}
