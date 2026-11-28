//! Reading the serialised Databricks tree: node kinds, payload fields and refusals.

use serde_json::{Map, Value};
use sqlquack_diagnostics::classification::models::{Construct, ConstructFamily};
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;

/// Fields that only record how the source was spelled or where it was.
const COSMETIC_FIELDS: &[&str] = &[
    "span",
    "keyword_span",
    "comma_spans",
    "alias_spans",
    "item_spans",
    "select_span",
    "list_head_span",
    "distinct_span",
    "row_limit_span",
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
    "alias_first",
    "explicit_asc",
    "double_colon_syntax",
    "integer_spelling",
    "written_as",
    "source_dialect",
];

pub(crate) type Payload = Map<String, Value>;

/// The kind and payload of a one-key node such as `{"add": {...}}`.
pub(crate) fn node_kind(node: &Value) -> Option<(&str, &Value)> {
    let object = node.as_object()?;
    match object.len() {
        1 => object
            .iter()
            .next()
            .map(|(kind, payload)| (kind.as_str(), payload)),
        _ => None,
    }
}

/// Whether a field holds its empty default.
fn is_inert(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::Array(items) => items.is_empty(),
        Value::Object(fields) => fields.is_empty(),
        Value::String(text) => text.is_empty(),
        Value::Number(number) => number.as_u64() == Some(0),
        Value::Bool(true) => false,
    }
}

/// The payload as an object whose fields are all `understood`, cosmetic or empty; else a refusal.
pub(crate) fn understood<'a>(
    payload: &'a Value,
    kind: &str,
    understood: &[&str],
) -> Result<&'a Payload, Refusal> {
    let Some(object) = payload.as_object() else {
        return Err(refused(kind, format!("the {kind} node has no fields")));
    };
    let unexpected: Vec<&str> = object
        .iter()
        .filter(|(field, value)| {
            !understood.contains(&field.as_str())
                && !COSMETIC_FIELDS.contains(&field.as_str())
                && !is_inert(value)
        })
        .map(|(field, _)| field.as_str())
        .collect();
    match unexpected.is_empty() {
        true => Ok(object),
        false => Err(refused(
            kind,
            format!(
                "no rule handles `{}` set on {kind}",
                unexpected.join("`, `")
            ),
        )),
    }
}

/// A missing-rule refusal of the node `kind`.
pub(crate) fn refused(kind: &str, reason: String) -> Refusal {
    missing(Construct::new(ConstructFamily::Node, kind), reason)
}

/// A refusal of the node `kind` whose rule covers it, for operands outside the rule's types.
pub(crate) fn outside(kind: &str, reason: String) -> Refusal {
    outside_conditions(Construct::new(ConstructFamily::Node, kind), reason)
}

/// A missing-rule refusal of the function `name`.
pub(crate) fn refused_function(name: &str, reason: String) -> Refusal {
    missing(Construct::new(ConstructFamily::Function, name), reason)
}

/// A refusal of the function `name` whose rule covers it, for arguments outside the rule's types.
pub(crate) fn outside_function(name: &str, reason: String) -> Refusal {
    outside_conditions(Construct::new(ConstructFamily::Function, name), reason)
}

/// A field's value, `Null` when absent.
pub(crate) fn field<'a>(payload: &'a Payload, name: &str) -> &'a Value {
    payload.get(name).unwrap_or(&Value::Null)
}

/// A list field's items, none when absent.
pub(crate) fn list<'a>(payload: &'a Payload, name: &str) -> &'a [Value] {
    payload
        .get(name)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Whether a boolean field is set.
pub(crate) fn flag(payload: &Payload, name: &str) -> bool {
    payload.get(name).and_then(Value::as_bool) == Some(true)
}

/// An identifier's name as written; quoted or not, Databricks compares names without case.
pub(crate) fn identifier(node: &Value) -> Result<String, Refusal> {
    let payload = understood(node, "identifier", &["name", "quoted"])?;
    match field(payload, "name").as_str() {
        Some(name) => Ok(name.to_owned()),
        None => Err(refused(
            "identifier",
            "the identifier has no name".to_owned(),
        )),
    }
}

/// The value of a string of ASCII digits, `None` when it is empty, holds another byte or overflows.
pub(crate) fn digits_value(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.bytes().try_fold(0u64, |value, byte| {
        value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))
    })
}

/// A DuckDB identifier for `name`, quoted so any spelling reads as written.
pub(crate) fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
