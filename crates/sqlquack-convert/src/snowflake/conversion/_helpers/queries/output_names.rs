//! The unaliased select items whose column names Snowflake receipts record.

use serde_json::Value;

use crate::snowflake::conversion::_helpers::support::locations::Locator;
use crate::snowflake::conversion::_helpers::support::tree::{
    LITERAL_DATE, LITERAL_NUMBER, LITERAL_STRING, LITERAL_TIMESTAMP, is_inert, list, node_kind,
    unexpected_fields,
};
use crate::snowflake::conversion::constants::DOLLAR_PARAMETER;

/// The field holding a cast's target type, which is named by its written text, not walked.
const CAST_TARGET_FIELD: &str = "to";
/// Functions and aggregates, called by name, whose unaliased column names receipts record.
const NAMED_CALLS: &[&str] = &[
    "ABS",
    "AVG",
    "CEIL",
    "COALESCE",
    "CONCAT",
    "COUNT",
    "DATEADD",
    "DATEDIFF",
    "DATE_TRUNC",
    "FLOOR",
    "IFF",
    "LENGTH",
    "LOWER",
    "MAX",
    "MIN",
    "MONTH",
    "NULLIF",
    "NVL",
    "ROUND",
    "SUBSTR",
    "SUM",
    "TO_DATE",
    "TO_VARCHAR",
    "TRIM",
    "UPPER",
    "YEAR",
];
/// Functions without arguments, named by their whole written text.
const NAMED_CLOCK_TEXTS: &[&str] = &["CURRENT_DATE()", "CURRENT_DATE", "CURRENT_TIMESTAMP()"];
/// Parsed node kinds whose unaliased names receipts record, each named by its written text.
const NAMED_KINDS: &[&str] = &[
    "add",
    "sub",
    "mul",
    "div",
    "mod",
    "concat",
    "eq",
    "neq",
    "lt",
    "lte",
    "gt",
    "gte",
    "and",
    "or",
    "paren",
    "neg",
    "pos",
    "not",
    "is_null",
    "cast",
    "coalesce",
    "nvl",
    "upper",
    "lower",
    "length",
    "trim",
    "year",
    "date_diff",
    "min",
    "max",
    "sum",
    "avg",
    "count",
    "try_cast",
    "case",
    "annotated",
];
/// How deep a named expression may nest before it refuses.
const MAX_NAMED_DEPTH: usize = 256;

/// Whether an unaliased item has a shape whose name receipts record; `text_of` gives a node's text.
pub(crate) fn recorded_name_shape(
    item: &Value,
    text_of: &dyn Fn(&Value) -> Option<String>,
) -> bool {
    named(item, text_of, MAX_NAMED_DEPTH)
}

/// Whether `node` and everything in it is named, as receipts record, by its upper-cased text.
fn named(node: &Value, text_of: &dyn Fn(&Value) -> Option<String>, depth: usize) -> bool {
    let Some((kind, payload)) = node_kind(node) else {
        return false;
    };
    if depth == 0 {
        return false;
    }
    let called = |names: &[&str]| {
        payload
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| names.contains(&name.to_ascii_uppercase().as_str()))
    };
    let upper_text = || text_of(node).map(|text| text.to_ascii_uppercase());
    let fields = || walked(payload, text_of, depth - 1);
    match kind {
        "column" => plain_column(node),
        "literal" => recorded_literal(node),
        "boolean" | "null" => true,
        "parameter" => {
            payload.get("style").and_then(Value::as_str) == Some(DOLLAR_PARAMETER)
                && payload.get("index").is_some_and(Value::is_u64)
        }
        "var" => payload
            .get("this")
            .and_then(Value::as_str)
            .is_some_and(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic())),
        "star" => payload
            .as_object()
            .is_none_or(|star| star.values().all(is_inert)),
        "function" if list_is_empty(payload) => {
            upper_text().is_some_and(|text| NAMED_CLOCK_TEXTS.contains(&text.as_str()))
        }
        "function" | "aggregate_function" => called(NAMED_CALLS) && fields(),
        "if_func" => upper_text().is_some_and(|text| text.starts_with("IFF(")) && fields(),
        named_kind if NAMED_KINDS.contains(&named_kind) => fields(),
        _ => false,
    }
}

/// Whether every node in `payload`'s fields is named, except a cast's target type and scalars.
fn walked(payload: &Value, text_of: &dyn Fn(&Value) -> Option<String>, depth: usize) -> bool {
    let Some(fields) = payload.as_object() else {
        return payload.is_null();
    };
    fields.iter().all(|(key, value)| match value {
        Value::Object(_) if key == CAST_TARGET_FIELD => true,
        Value::Object(_) | Value::Array(_) => all_named(value, text_of, depth),
        _ => true,
    })
}

/// Whether `value`, a node or a list of nodes and lists such as CASE's WHEN pairs, is named.
fn all_named(value: &Value, text_of: &dyn Fn(&Value) -> Option<String>, depth: usize) -> bool {
    match value {
        Value::Array(items) => items.iter().all(|item| all_named(item, text_of, depth)),
        Value::String(_) => true,
        _ => named(value, text_of, depth),
    }
}

/// Whether a call's argument list is absent or empty.
fn list_is_empty(payload: &Value) -> bool {
    payload
        .as_object()
        .is_none_or(|fields| list(fields, "args").is_empty())
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

/// The written text of an unaliased `v:key`, an unquoted column and one plain key, as recorded.
pub(crate) fn variant_key_text(item: &Value, locator: &Locator<'_>) -> Option<String> {
    let ("j_s_o_n_extract", payload) = node_kind(item)? else {
        return None;
    };
    let fields = payload.as_object()?;
    if !unexpected_fields(fields, &["this", "expression", "variant_extract"]).is_empty() {
        return None;
    }
    let base = payload.get("this")?;
    let ("column", column) = node_kind(base)? else {
        return None;
    };
    let unquoted = column
        .get("name")
        .is_some_and(|name| name.get("quoted").and_then(Value::as_bool) != Some(true));
    if !plain_column(base) || !unquoted || column.get("table").is_some_and(|t| !t.is_null()) {
        return None;
    }
    let key = payload
        .get("expression")?
        .get("literal")?
        .get("value")?
        .as_str()?;
    let mut characters = key.chars();
    let plain_key = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_');
    let path = format!(":{key}");
    if !plain_key || locator.following(base, path.chars().count())? != path {
        return None;
    }
    Some(format!("{}{path}", locator.source_text(base)?))
}

/// A number in any spelling, which receipts show named as written, or printable ASCII text.
fn recorded_literal(node: &Value) -> bool {
    let Some(("literal", payload)) = node_kind(node) else {
        return false;
    };
    let value = payload.get("value").and_then(Value::as_str);
    match payload.get("literal_type").and_then(Value::as_str) {
        Some(LITERAL_NUMBER) => value.is_some_and(|value| !value.is_empty()),
        Some(LITERAL_STRING) => value.is_some_and(|value| {
            value
                .bytes()
                .all(|byte| (b' '..=b'~').contains(&byte) && byte != b'\\')
        }),
        Some(LITERAL_DATE | LITERAL_TIMESTAMP) => value.is_some_and(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| (b' '..=b'~').contains(&byte) && byte != b'\'' && byte != b'\\')
        }),
        _ => false,
    }
}
