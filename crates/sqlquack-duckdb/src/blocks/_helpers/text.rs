//! Text helpers the building blocks share.

use sqlquack_core::failures::constants::{
    NUMBER_OUT_OF_RANGE, NUMBER_TEXT_OUT_OF_RANGE, VARIANT_NOT_CAST,
};
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::failures::models::SnowflakeCode;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{CONVERSION_WHITESPACE, NUMBER_RANGE_MESSAGE, VARIANT_CAST_MESSAGE};
use crate::blocks::main::raised::raised;
use crate::blocks::main::string_literal::string_literal;
use crate::blocks::models::TextFailure;

pub(crate) fn expanded(sql: &str, expansions: &[(&str, &str)], log: &ProvenanceLog) -> String {
    let mut text = sql.to_owned();
    for (from, to) in expansions {
        text = format!(
            "replace({text}, {}, {})",
            string_literal(from, log),
            string_literal(to, log)
        );
    }
    text
}

/// What a failed conversion yields: NULL when trying, else text's error `code` or a VARIANT's.
pub(crate) fn conversion_failure(
    failure: TextFailure,
    (code, message): (SnowflakeCode, &str),
) -> String {
    match failure {
        TextFailure::Null => "NULL".to_owned(),
        TextFailure::Text => tagged_error(code, message),
        TextFailure::Variant => tagged_error(VARIANT_NOT_CAST, VARIANT_CAST_MESSAGE),
    }
}

/// What numeric text beyond the target's range yields: NULL when trying, else Snowflake's error.
pub(crate) fn overflow_failure(failure: TextFailure) -> String {
    match failure {
        TextFailure::Null => "NULL".to_owned(),
        TextFailure::Text => tagged_error(NUMBER_TEXT_OUT_OF_RANGE, NUMBER_RANGE_MESSAGE),
        TextFailure::Variant => tagged_error(NUMBER_OUT_OF_RANGE, NUMBER_RANGE_MESSAGE),
    }
}

/// An expression raising the Snowflake error `code`, tagged so verification can read it back.
pub(crate) fn tagged_error(code: SnowflakeCode, message: &str) -> String {
    raised(&tagged_message(code, message))
}

/// The text with surrounding whitespace removed, as Snowflake's conversions read it.
pub(crate) fn space_trimmed(sql: &str) -> String {
    format!("trim({sql}, {CONVERSION_WHITESPACE})")
}

/// A quoted, comma-separated list of words for `IN (...)`.
pub(crate) fn word_list(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| format!("'{word}'"))
        .collect::<Vec<String>>()
        .join(", ")
}
