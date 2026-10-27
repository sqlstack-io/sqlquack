//! Text handling for timestamp parsing.

use sqlquack_core::failures::constants::VARIANT_NOT_CAST;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::failures::models::SnowflakeCode;

use crate::blocks::constants::{CONVERSION_WHITESPACE, VARIANT_CAST_MESSAGE};
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;

/// `NULL` when trying, else an expression raising text's Snowflake error `code` or a VARIANT's.
pub(crate) fn parse_failure(
    failure: TextFailure,
    (code, message): (SnowflakeCode, &str),
) -> String {
    let (code, message) = match failure {
        TextFailure::Null => return "NULL".to_owned(),
        TextFailure::Text => (code, message),
        TextFailure::Variant => (VARIANT_NOT_CAST, VARIANT_CAST_MESSAGE),
    };
    raised(&tagged_message(code, message))
}

/// `converted`, or `failure` where it is NULL for non-NULL text: impossible values in a spelling.
pub(crate) fn checked(sql: &str, converted: &str, failure: &str) -> String {
    format!("coalesce({converted}, CASE WHEN {sql} IS NULL THEN NULL ELSE {failure} END)")
}

/// The text with the whitespace Snowflake's conversions ignore removed.
pub(crate) fn whitespace_trimmed(sql: &str) -> String {
    format!("trim({sql}, {CONVERSION_WHITESPACE})")
}
