//! Text handling for timestamp parsing.

use sqlquack_diagnostics::failures::constants::VARIANT_NOT_CAST;
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::failures::models::SnowflakeCode;

use crate::blocks::constants::{
    CONVERSION_WHITESPACE, DIGITLESS_TEXT_PATTERN, VARIANT_CAST_MESSAGE,
};
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

/// `body` over `value` evaluated once, as the lambda parameter `name`.
pub(crate) fn bound(value: &str, name: &str, body: &str) -> String {
    format!("list_transform([{value}], lambda {name}: {body})[1]")
}

/// Whether `sql` has no ASCII digit, which TRY_ parsing was recorded reading as NULL.
pub(crate) fn digitless(sql: &str) -> String {
    format!("regexp_full_match({sql}, '{DIGITLESS_TEXT_PATTERN}')")
}
