//! Text helpers the building blocks share.

use sqlquack_diagnostics::failures::constants::{
    NUMBER_OUT_OF_RANGE, NUMBER_TEXT_OUT_OF_RANGE, VARIANT_NOT_CAST,
};
use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;
use sqlquack_diagnostics::failures::models::SnowflakeCode;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    CONVERSION_WHITESPACE, INLINE_ARGUMENT_LIMIT, NUMBER_RANGE_MESSAGE, VARIANT_CAST_MESSAGE,
};
use crate::blocks::main::string_literal::string_literal;
use crate::blocks::models::TextFailure;
use crate::raises::main::raised::raised;

/// `sql` with each one-character key of `expansions` spelled as its text, by a single lookup.
pub(crate) fn expanded(sql: &str, expansions: &[(&str, &str)], log: &ProvenanceLog) -> String {
    let class: String = expansions
        .iter()
        .flat_map(|(from, _)| from.chars())
        .map(|character| format!("\\x{{{:x}}}", u32::from(character)))
        .collect();
    let list = |texts: Vec<&str>| {
        texts
            .into_iter()
            .map(|text| string_literal(text, log))
            .collect::<Vec<String>>()
            .join(", ")
    };
    let keys = list(expansions.iter().map(|(from, _)| *from).collect());
    let values = list(expansions.iter().map(|(_, to)| *to).collect());
    format!(
        "list_transform([{sql}], lambda case_text: CASE WHEN regexp_matches(case_text, '[{class}]') \
         THEN array_to_string(list_transform(string_split(case_text, ''), lambda case_character: \
         coalesce([{values}][list_position([{keys}], case_character)], case_character)), '') \
         ELSE case_text END)[1]"
    )
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

/// `body` over `sql`, which a long `sql` reaches once through the lambda parameter `name`.
pub(crate) fn once_bound(sql: &str, name: &str, body: impl Fn(&str) -> String) -> String {
    match sql.len() > INLINE_ARGUMENT_LIMIT {
        true => format!("list_transform([{sql}], lambda {name}: {})[1]", body(name)),
        false => body(sql),
    }
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
