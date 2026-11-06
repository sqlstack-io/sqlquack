//! Reading the serialised source tree: node kinds, payload fields and refusals.

use crate::conversion::_helpers::support::collecting::{collecting, events, record, skipped};
use std::collections::BTreeMap;

use serde_json::{Map, Value};
use sqlquack_core::classification::constants::{
    PROJECT_FUNCTION_CONSTRUCT, PROJECT_FUNCTION_PLACEHOLDER,
};
use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_semantics::dialects::constants::SNOWFLAKE;
use sqlquack_semantics::dialects::main::identifier_name::identifier_name as resolved_identifier;

use crate::conversion::models::ConversionOptions;

/// Fields that only record how the source was spelled or where it was.
const COSMETIC_FIELDS: &[&str] = &[
    "span",
    "trailing_comments",
    "comments",
    "leading_comments",
    "pre_alias_comments",
    "left_comments",
    "operator_comments",
    "right_comments",
    "inferred_type",
    "alias_explicit_as",
    "alias_keyword",
    "explicit_asc",
    "double_colon_syntax",
    "original_name",
    "postfix_form",
    "use_inner_keyword",
    "use_outer_keyword",
    "kind_text",
    "start_side_text",
    "end_side_text",
    "alias_first",
    "position_explicit",
    "nesting_group",
    "integer_spelling",
    "real_spelling",
    "parenthesized_length",
];

pub(crate) type Payload = Map<String, Value>;

/// The parser's literal type for numbers.
pub(crate) const LITERAL_NUMBER: &str = "number";
/// The parser's literal type for strings.
pub(crate) const LITERAL_STRING: &str = "string";

/// The kind and payload of a one-key node such as `{"add": {...}}`.
pub(crate) fn node_kind(node: &Value) -> Option<(&str, &Value)> {
    let object = node.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object
        .iter()
        .next()
        .map(|(kind, payload)| (kind.as_str(), payload))
}

/// Whether a field holds its empty default.
pub(crate) fn is_inert(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::Array(items) => items.is_empty(),
        Value::Object(fields) => fields.is_empty(),
        Value::String(text) => text.is_empty(),
        Value::Number(number) => number.as_u64() == Some(0),
        Value::Bool(true) => false,
    }
}

/// The fields a payload sets beyond `understood`, cosmetic and inert ones aside.
pub(crate) fn unexpected_fields<'a>(payload: &'a Payload, understood: &[&str]) -> Vec<&'a str> {
    payload
        .iter()
        .filter(|(field, value)| {
            !understood.contains(&field.as_str())
                && !COSMETIC_FIELDS.contains(&field.as_str())
                && !is_inert(value)
        })
        .map(|(field, _)| field.as_str())
        .collect()
}

/// Refuses when a payload sets a field the rule does not understand.
pub(crate) fn require_understood(
    payload: &Payload,
    understood: &[&str],
    construct: &Construct,
) -> Result<(), Refusal> {
    let unexpected = unexpected_fields(payload, understood);
    if unexpected.is_empty() {
        return Ok(());
    }
    let refusal = missing(
        construct.clone(),
        format!(
            "no rule handles `{}` set on {}",
            unexpected.join("`, `"),
            construct.key()
        ),
    );
    if collecting() {
        record(refusal, events());
        skipped();
        return Ok(());
    }
    Err(refusal)
}

/// A payload as an object, or a refusal naming the construct.
pub(crate) fn payload_object<'a>(
    payload: &'a Value,
    construct: &Construct,
) -> Result<&'a Payload, Refusal> {
    payload.as_object().ok_or_else(|| {
        missing(
            construct.clone(),
            format!("{} has an unexpected shape", construct.key()),
        )
    })
}

/// A field that must be present and not null.
pub(crate) fn field<'a>(
    payload: &'a Payload,
    name: &str,
    construct: &Construct,
) -> Result<&'a Value, Refusal> {
    payload
        .get(name)
        .filter(|value| !value.is_null())
        .ok_or_else(|| {
            missing(
                construct.clone(),
                format!("{} lacks `{name}`", construct.key()),
            )
        })
}

/// A list field; absent or null reads as empty.
pub(crate) fn list<'a>(payload: &'a Payload, name: &str) -> &'a [Value] {
    payload
        .get(name)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// A boolean field; absent or null reads as false.
pub(crate) fn flag(payload: &Payload, name: &str) -> bool {
    payload.get(name).and_then(Value::as_bool).unwrap_or(false)
}

/// The construct for a node kind.
pub(crate) fn node_construct(kind: &str) -> Construct {
    Construct::new(ConstructFamily::Node, kind)
}

/// The construct for a generic function call.
pub(crate) fn function_construct(name: &str) -> Construct {
    if name
        .to_ascii_lowercase()
        .starts_with(PROJECT_FUNCTION_PLACEHOLDER)
    {
        return Construct::new(ConstructFamily::Function, PROJECT_FUNCTION_CONSTRUCT);
    }
    Construct::new(ConstructFamily::Function, name.to_ascii_uppercase())
}

/// An identifier's resolved name in Snowflake: upper case unless quoted.
pub(crate) fn identifier_name(identifier: &Value) -> Option<String> {
    resolved_identifier(identifier, &SNOWFLAKE)
}

/// The text of an integer literal node.
pub(crate) fn integer_literal(node: &Value) -> Option<u64> {
    let ("literal", payload) = node_kind(node)? else {
        return None;
    };
    let LITERAL_NUMBER = payload.get("literal_type")?.as_str()? else {
        return None;
    };
    let digits = payload.get("value")?.as_str()?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.bytes().try_fold(0u64, |value, byte| {
        value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))
    })
}

/// The tree conversion reads, positions stripped and quoted identifiers folded if case is ignored.
pub(crate) fn session_tree(
    positioned: &Value,
    options: &ConversionOptions,
) -> Result<Value, String> {
    let mut tree = strip_positions(positioned);
    if options.quoted_identifiers_ignore_case {
        fold_quoted(&mut tree)?;
    }
    if !options.user_types.is_empty() {
        tree = with_user_types(&tree, &options.user_types);
    }
    Ok(tree)
}

/// `value` with each type named for a user-defined type replaced by the base type it stands for.
pub(crate) fn with_user_types(value: &Value, types: &BTreeMap<String, Value>) -> Value {
    if let Some(base) = user_type(value, types) {
        return base.clone();
    }
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, field)| (key.clone(), with_user_types(field, types)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| with_user_types(item, types))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The base type of a custom type spelled as a plain name that setup CREATE TYPE'd.
fn user_type<'a>(value: &Value, types: &'a BTreeMap<String, Value>) -> Option<&'a Value> {
    let fields = value.as_object()?;
    if fields.get("data_type").and_then(Value::as_str) != Some("custom") {
        return None;
    }
    let name = fields.get("name")?.as_str()?;
    let plain = name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_');
    plain.then(|| types.get(&name.to_ascii_uppercase()))?
}

/// Upper-cases the name of each quoted identifier under `value`.
fn fold_quoted(value: &mut Value) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            if fields.get("quoted").and_then(Value::as_bool) == Some(true)
                && let Some(Value::String(name)) = fields.get_mut("name")
            {
                if !name.is_ascii() {
                    return Err(format!(
                        "the quoted identifier \"{name}\" holds a non-ASCII character, whose case \
                         folding under QUOTED_IDENTIFIERS_IGNORE_CASE has no recording"
                    ));
                }
                *name = name.to_ascii_uppercase();
            }
            fields.values_mut().try_for_each(fold_quoted)
        }
        Value::Array(items) => items.iter_mut().try_for_each(fold_quoted),
        _ => Ok(()),
    }
}

/// The tree without positions and comments, which no rule reads.
pub(crate) fn strip_positions(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .filter(|(field, _)| {
                    !matches!(
                        field.as_str(),
                        "span" | "trailing_comments" | "leading_comments" | "inferred_type"
                    )
                })
                .map(|(field, child)| (field.clone(), strip_positions(child)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_positions).collect()),
        other => other.clone(),
    }
}
