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
/// The join kind prefix of NATURAL joins, which compare every column the two sides share.
const NATURAL_JOIN_KIND: &str = "Natural";
/// The fields of an identifier node: its name and whether it was quoted.
const IDENTIFIER_FIELDS: [&str; 2] = ["name", "quoted"];
/// The field holding a call's arguments.
const ARGUMENTS_FIELD: &str = "args";

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

/// Refuses when a payload sets a field the rule does not understand.
pub(crate) fn require_understood(
    payload: &Payload,
    understood: &[&str],
    construct: &Construct,
) -> Result<(), Refusal> {
    let unexpected: Vec<&str> = payload
        .iter()
        .filter(|(field, value)| {
            !understood.contains(&field.as_str())
                && !COSMETIC_FIELDS.contains(&field.as_str())
                && !is_inert(value)
        })
        .map(|(field, _)| field.as_str())
        .collect();
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
    let mut tree = strip_positions(positioned);
    if ignore_quoted_case {
        fold_quoted(&mut tree)?;
    }
    Ok(tree)
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

/// Whether `tree` may read column `name`: by name, by a star other than COUNT(*)'s, or by NATURAL.
pub(crate) fn may_read_column(tree: &Value, name: &str) -> bool {
    match tree {
        Value::Array(items) => items.iter().any(|item| may_read_column(item, name)),
        Value::Object(fields) => {
            if fields.len() == IDENTIFIER_FIELDS.len()
                && IDENTIFIER_FIELDS
                    .iter()
                    .all(|field| fields.contains_key(*field))
            {
                return identifier_name(tree).is_some_and(|resolved| resolved == name);
            }
            match node_kind(tree) {
                Some(("star", _)) => return true,
                Some(("aggregate_function", payload)) if counts_rows(payload) => {
                    return payload.as_object().is_some_and(|call| {
                        call.iter().any(|(field, value)| {
                            field != ARGUMENTS_FIELD && may_read_column(value, name)
                        })
                    });
                }
                _ => {}
            }
            if fields
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.starts_with(NATURAL_JOIN_KIND))
                && fields.contains_key("using")
            {
                return true;
            }
            fields.values().any(|value| may_read_column(value, name))
        }
        _ => false,
    }
}

/// Whether an aggregate call is COUNT(*) with a bare star, which counts rows and reads no column.
fn counts_rows(payload: &Value) -> bool {
    let named_count = payload
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|called| called.eq_ignore_ascii_case("count"));
    let bare_star = match payload
        .get("args")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
    {
        Some([argument]) => match node_kind(argument) {
            Some(("star", star)) => star
                .as_object()
                .is_some_and(|star| star.values().all(Value::is_null)),
            _ => false,
        },
        _ => false,
    };
    named_count && bare_star
}
