//! The unaliased select items whose column names Snowflake receipts record.

use serde_json::Value;

use crate::conversion::_helpers::support::tree::{
    LITERAL_NUMBER, LITERAL_STRING, list, node_kind, unexpected_fields,
};
use crate::conversion::constants::{
    OUTPUT_NAMED_AGGREGATES, OUTPUT_NAMED_ARITHMETIC, OUTPUT_NAMED_CAST_ENDINGS,
    OUTPUT_NAMED_CURRENT_TIME, OUTPUT_NAMED_FUNCTIONS, OUTPUT_NAMED_ROUND,
};

/// Whether an unaliased item written as `text` has a shape whose name receipts record.
pub(crate) fn recorded_name_shape(item: &Value, text: &str) -> bool {
    let Some((kind, payload)) = node_kind(item) else {
        return false;
    };
    let Some(fields) = payload.as_object() else {
        return matches!(kind, "null");
    };
    let only = |understood: &[&str]| unexpected_fields(fields, understood).is_empty();
    let named = |names: &[&str]| {
        payload
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| names.contains(&name.to_ascii_uppercase().as_str()))
    };
    let arguments = list(fields, "args");
    match kind {
        "aggregate_function" => {
            only(&["name", "args", "distinct"])
                && named(OUTPUT_NAMED_AGGREGATES)
                && arguments.len() == 1
                && arguments.iter().all(|argument| {
                    plain_column(argument)
                        || (named(&["COUNT"]) && matches!(node_kind(argument), Some(("star", _))))
                })
        }
        "function" if arguments.is_empty() => {
            only(&["name", "args", "no_parens"])
                && OUTPUT_NAMED_CURRENT_TIME.contains(&text.to_ascii_uppercase().as_str())
        }
        "function" if named(&[OUTPUT_NAMED_ROUND]) => {
            only(&["name", "args"])
                && arguments.get(2).is_none()
                && arguments.first().is_some_and(plain_column)
                && arguments.iter().skip(1).all(integer_literal)
        }
        "function" => {
            only(&["name", "args"])
                && named(OUTPUT_NAMED_FUNCTIONS)
                && !arguments.is_empty()
                && arguments
                    .iter()
                    .all(|argument| plain_column(argument) || integer_literal(argument))
        }
        "concat" => {
            only(&["left", "right"])
                && payload.get("left").is_some_and(plain_column)
                && payload.get("right").is_some_and(string_literal)
        }
        "cast" => {
            let upper = text.to_ascii_uppercase();
            only(&["this", "to", "double_colon_syntax"])
                && payload.get("this").is_some_and(plain_column)
                && OUTPUT_NAMED_CAST_ENDINGS
                    .iter()
                    .any(|ending| upper.ends_with(ending))
        }
        "neg" => {
            only(&["this"])
                && payload.get("this").is_some_and(integer_literal)
                && text.strip_prefix('-').is_some_and(|rest| {
                    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
                })
        }
        "literal" => recorded_literal(item),
        "null" | "boolean" => true,
        arithmetic if OUTPUT_NAMED_ARITHMETIC.contains(&arithmetic) => {
            only(&["left", "right"])
                && payload.get("left").is_some_and(plain_column)
                && payload.get("right").is_some_and(integer_literal)
        }
        _ => false,
    }
}

/// A column, qualified or not, each part unquoted or quoted and already upper case.
fn plain_column(node: &Value) -> bool {
    let Some(("column", payload)) = node_kind(node) else {
        return false;
    };
    let Some(fields) = payload.as_object() else {
        return false;
    };
    let upper_or_unquoted = |part: &Value| {
        let quoted = part.get("quoted").and_then(Value::as_bool) == Some(true);
        let name = part.get("name").and_then(Value::as_str).unwrap_or_default();
        !name.is_empty() && (!quoted || name == name.to_ascii_uppercase())
    };
    unexpected_fields(fields, &["name", "table"]).is_empty()
        && payload.get("name").is_some_and(upper_or_unquoted)
        && payload
            .get("table")
            .filter(|table| !table.is_null())
            .is_none_or(upper_or_unquoted)
}

/// A literal spelled as a receipt records: an integer, a decimal, an exponent or plain text.
fn recorded_literal(node: &Value) -> bool {
    let Some(("literal", payload)) = node_kind(node) else {
        return false;
    };
    let value = payload
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match payload.get("literal_type").and_then(Value::as_str) {
        Some(LITERAL_NUMBER) => {
            let (mantissa, exponent) = match value.split_once('e') {
                Some((mantissa, exponent)) => (mantissa, Some(exponent)),
                None => (value, None),
            };
            let (whole, fraction) = match mantissa.split_once('.') {
                Some((whole, fraction)) => (whole, Some(fraction)),
                None => (mantissa, None),
            };
            let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
            let leading_zero = whole.starts_with('0');
            digits(whole)
                && (whole.len() == 1 || !leading_zero)
                && fraction.is_none_or(digits)
                && exponent
                    .is_none_or(|exponent| fraction.is_none() && !leading_zero && digits(exponent))
        }
        Some(LITERAL_STRING) => string_literal(node),
        _ => false,
    }
}

fn integer_literal(node: &Value) -> bool {
    recorded_literal(node)
        && node_kind(node)
            .and_then(|(_, payload)| payload.get("value"))
            .and_then(Value::as_str)
            .is_some_and(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Non-empty printable ASCII text without a quote or backslash, which has one spelling.
fn string_literal(node: &Value) -> bool {
    let Some(("literal", payload)) = node_kind(node) else {
        return false;
    };
    payload.get("literal_type").and_then(Value::as_str) == Some(LITERAL_STRING)
        && payload
            .get("value")
            .and_then(Value::as_str)
            .is_some_and(|value| {
                !value.is_empty()
                    && value
                        .bytes()
                        .all(|byte| (b' '..=b'~').contains(&byte) && byte != b'\'' && byte != b'\\')
            })
}
