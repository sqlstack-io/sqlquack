//! `single_expression`: whether DuckDB's parser reads a text as exactly one expression.

use duckdb::Connection;
use serde_json::Value;

use crate::replay::constants::{EXPRESSION_TEMPLATE, SELECT_ITEMS};

/// Ok when `SELECT (<text>)` parses as `SELECT (NULL)` does but for its one select item; never run.
pub fn single_expression(text: &str) -> Result<(), String> {
    let connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    let parsed = |sql: &str| -> Result<Value, String> {
        let serialized: String = connection
            .query_row("SELECT json_serialize_sql(?)", [sql], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        serde_json::from_str(&serialized).map_err(|error| error.to_string())
    };
    let template = select_node(&parsed(EXPRESSION_TEMPLATE)?)
        .ok_or_else(|| "DuckDB does not serialise SELECT (NULL)".to_owned())?;
    let candidate = parsed(&format!("SELECT ({text})"))?;
    let node = select_node(&candidate).ok_or_else(|| {
        let message = candidate
            .get("error_message")
            .and_then(Value::as_str)
            .unwrap_or("more than one statement");
        format!("not one DuckDB expression: {message}")
    })?;
    let items = node
        .get(SELECT_ITEMS)
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let without_items = |mut node: Value| {
        if let Some(fields) = node.as_object_mut() {
            fields.remove(SELECT_ITEMS);
        }
        node
    };
    match items == 1 && without_items(node) == without_items(template) {
        true => Ok(()),
        false => Err("not one DuckDB expression: it reaches past its parentheses".to_owned()),
    }
}

/// The one statement's node of a serialised parse, or None for an error or several statements.
fn select_node(parsed: &Value) -> Option<Value> {
    match parsed.get("statements").and_then(Value::as_array) {
        Some(statements) if statements.len() == 1 => statements[0].get("node").cloned(),
        _ => None,
    }
}
