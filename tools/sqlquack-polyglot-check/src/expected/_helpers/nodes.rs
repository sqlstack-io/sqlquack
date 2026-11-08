//! Trees that differ only by a wrapper node or a field SQLQuack keeps and polyglot drops or infers.

use serde_json::Value;

use crate::expected::constants::{
    ALL_KEY, ALL_TEXT, COMMAND_TREE, EXPRESSIONS_KEY, OK_PREFIX, RAW_TREE, THIS_KEY, VAR_KEY,
};

/// Whether both rendered parses are trees that are equal once SQLQuack's `node` wrappers go.
pub(crate) fn differ_only_by_kept_node(polyglot: &str, sqlquack: &str, node: &str) -> bool {
    let (Some(theirs), Some(ours)) = (
        polyglot.strip_prefix(OK_PREFIX),
        sqlquack.strip_prefix(OK_PREFIX),
    ) else {
        return false;
    };
    match (
        serde_json::from_str::<Value>(theirs),
        serde_json::from_str::<Value>(ours),
    ) {
        (Ok(theirs), Ok(ours)) => theirs != ours && without_node(ours, node) == theirs,
        (Err(_), _) | (_, Err(_)) => false,
    }
}

/// `value` with every `{ node: { "this": inner } }` replaced by `inner`.
fn without_node(value: Value, node: &str) -> Value {
    match value {
        Value::Object(mut object) => {
            let wrapped: bool = object.len() == 1
                && object
                    .get(node)
                    .and_then(Value::as_object)
                    .is_some_and(|inner| inner.len() == 1 && inner.contains_key(THIS_KEY));
            match object.remove(node).filter(|_| wrapped) {
                Some(Value::Object(mut inner)) => {
                    without_node(inner.remove(THIS_KEY).unwrap_or(Value::Null), node)
                }
                Some(other) => other,
                None => Value::Object(
                    object
                        .into_iter()
                        .map(|(key, value)| (key, without_node(value, node)))
                        .collect(),
                ),
            }
        }
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|value| without_node(value, node))
                .collect(),
        ),
        other => other,
    }
}

/// Whether both rendered parses are trees that differ, and are equal once every `field` key goes from each.
pub(crate) fn differ_only_by_kept_field(polyglot: &str, sqlquack: &str, field: &str) -> bool {
    let (Some(theirs), Some(ours)) = (
        polyglot.strip_prefix(OK_PREFIX),
        sqlquack.strip_prefix(OK_PREFIX),
    ) else {
        return false;
    };
    match (
        serde_json::from_str::<Value>(theirs),
        serde_json::from_str::<Value>(ours),
    ) {
        (Ok(theirs), Ok(ours)) => {
            theirs != ours && without_field(theirs, field) == without_field(ours, field)
        }
        (Err(_), _) | (_, Err(_)) => false,
    }
}

/// `value` with every object key `field` removed.
fn without_field(value: Value, field: &str) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .filter(|(key, _)| key != field)
                .map(|(key, value)| (key, without_field(value, field)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|value| without_field(value, field))
                .collect(),
        ),
        other => other,
    }
}

/// Whether polyglot's rendered tree is one opaque command or one statement kept as raw SQL.
pub(crate) fn is_opaque(polyglot: &str) -> bool {
    [COMMAND_TREE, RAW_TREE]
        .iter()
        .any(|opaque| polyglot.starts_with(&format!("{OK_PREFIX}{opaque}")))
}

/// Whether both rendered parses are trees that differ, and are equal once SQLQuack's `clause` ALL is polyglot's key.
pub(crate) fn differ_only_by_kept_all(polyglot: &str, sqlquack: &str, clause: &str) -> bool {
    let (Some(theirs), Some(ours)) = (
        polyglot.strip_prefix(OK_PREFIX),
        sqlquack.strip_prefix(OK_PREFIX),
    ) else {
        return false;
    };
    match (
        serde_json::from_str::<Value>(theirs),
        serde_json::from_str::<Value>(ours),
    ) {
        (Ok(theirs), Ok(ours)) => theirs != ours && with_all_as_key(ours, clause) == theirs,
        (Err(_), _) | (_, Err(_)) => false,
    }
}

/// `value` with each `clause` of no keys and an `all` rewritten as one key `ALL` carrying its options.
fn with_all_as_key(value: Value, clause: &str) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    let value = match (key == clause, value) {
                        (true, Value::Object(fields)) => all_as_key(fields),
                        (_, value) => value,
                    };
                    (key, with_all_as_key(value, clause))
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|value| with_all_as_key(value, clause))
                .collect(),
        ),
        other => other,
    }
}

/// A clause's fields, its `all` rewritten as one key `ALL` carrying its options when it has no keys.
fn all_as_key(mut fields: serde_json::Map<String, Value>) -> Value {
    if fields.get(EXPRESSIONS_KEY) != Some(&Value::Array(Vec::new())) {
        return Value::Object(fields);
    }
    match fields.remove(ALL_KEY) {
        Some(Value::Object(mut options)) => {
            options.insert(
                THIS_KEY.to_owned(),
                serde_json::json!({ VAR_KEY: { THIS_KEY: ALL_TEXT } }),
            );
            fields.insert(
                EXPRESSIONS_KEY.to_owned(),
                Value::Array(vec![Value::Object(options)]),
            );
        }
        Some(other) => {
            fields.insert(ALL_KEY.to_owned(), other);
        }
        None => {}
    }
    Value::Object(fields)
}
