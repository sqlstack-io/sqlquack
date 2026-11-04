//! Trees that differ only by a wrapper node SQLQuack keeps and polyglot drops.

use serde_json::Value;

use crate::expected::constants::{OK_PREFIX, THIS_KEY};

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
