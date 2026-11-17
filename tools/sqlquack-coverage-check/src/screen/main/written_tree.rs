//! `written_tree`: the part of a parse tree that records what the text wrote.

use serde_json::Value;
use sqlquack_syntax::Expression;

use crate::screen::constants::IGNORED_KEYS;

fn without_ignored(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(key, _)| !IGNORED_KEYS.contains(&key.as_str()))
                .map(|(key, child)| (key, without_ignored(child)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(without_ignored).collect()),
        other => other,
    }
}

/// The statements as JSON without source positions or trailing comments, or why they cannot be.
pub fn written_tree(statements: &[Expression]) -> Value {
    serde_json::to_value(statements).map_or_else(
        |error| Value::String(format!("unserialisable tree: {error}")),
        without_ignored,
    )
}
