//! `snowflake_kind`: the kind of a recorded Snowflake error.

use crate::failures::constants::SNOWFLAKE_ERRORS;
use crate::failures::models::ErrorKind;

/// The kind of the Snowflake error with this number and SQLSTATE, when the taxonomy knows it.
pub fn snowflake_kind(errno: i64, sqlstate: &str) -> Option<ErrorKind> {
    SNOWFLAKE_ERRORS
        .iter()
        .find(|(code, _)| code.errno == errno && code.sqlstate == sqlstate)
        .map(|(_, kind)| *kind)
}
