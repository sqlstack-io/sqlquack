//! Reading the serialised source tree: node kinds, payload fields and refusals.

use crate::conversion::_helpers::support::collecting::{collecting, events, record, skipped};
use serde_json::{Map, Value};
use sqlquack_core::classification::constants::{
    PROJECT_FUNCTION_CONSTRUCT, PROJECT_FUNCTION_PLACEHOLDER,
};
use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_semantics::dialects::constants::SNOWFLAKE;
use sqlquack_semantics::dialects::main::identifier_name::identifier_name as resolved_identifier;

/// Fields that only record how the source was spelled or where it was.
const COSMETIC_FIELDS: &[&str] = &[
    "span",
    "trailing_comments",
    "comments",
    "leading_comments",
    "pre_alias_comments",
    "post_select_comments",
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
pub(crate) fn session_tree(positioned: &Value, ignore_quoted_case: bool) -> Result<Value, String> {
    let mut tree = without_comment_wrappers(&strip_positions(positioned));
    if ignore_quoted_case {
        fold_quoted(&mut tree)?;
    }
    Ok(tree)
}

/// `value` with each node that only wrapped a comment replaced by the node it wrapped.
///
/// A select item's own wrapper stays: the comment is part of an unaliased item's written text,
/// which names its column, so the projection rule reads the wrapper itself.
fn without_comment_wrappers(value: &Value) -> Value {
    unwrap_comments(value, false)
}

fn unwrap_comments(value: &Value, keep_wrapper: bool) -> Value {
    match value {
        Value::Object(fields) => match fields.get("annotated").and_then(Value::as_object) {
            Some(annotated) if !keep_wrapper && fields.len() == 1 && annotated.len() == 1 => {
                annotated
                    .get("this")
                    .map_or_else(|| value.clone(), without_comment_wrappers)
            }
            _ => {
                let select = fields.len() == 1 && fields.contains_key("select");
                Value::Object(
                    fields
                        .iter()
                        .map(|(key, field)| {
                            let field = match (select, field.as_object()) {
                                (true, Some(payload)) => Value::Object(
                                    payload
                                        .iter()
                                        .map(|(name, clause)| {
                                            (name.clone(), unwrap_clause(name, clause))
                                        })
                                        .collect(),
                                ),
                                _ => without_comment_wrappers(field),
                            };
                            (key.clone(), field)
                        })
                        .collect(),
                )
            }
        },
        Value::Array(items) => Value::Array(items.iter().map(without_comment_wrappers).collect()),
        other => other.clone(),
    }
}

/// A SELECT clause with comment wrappers unwrapped, except around the select items themselves.
fn unwrap_clause(name: &str, clause: &Value) -> Value {
    match (name, clause) {
        ("expressions", Value::Array(items)) => Value::Array(
            items
                .iter()
                .map(|item| unwrap_comments(item, true))
                .collect(),
        ),
        _ => without_comment_wrappers(clause),
    }
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
