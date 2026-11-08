//! `compare_located`: a located parse against the faithful parse it must equal, spans aside.

use serde_json::Value;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions, parse_statements_with};

use crate::comparison::constants::SPAN_FIELD;

/// How the located parse of `sql` differs from its faithful parse beyond spans, if it does.
pub fn compare_located(sql: &str, dialect: Dialect) -> Option<String> {
    let parse = |mode: CallMode| -> Result<Value, String> {
        parse_statements_with(sql, dialect, &ParseOptions::default(), mode)
            .map_err(|error| format!("{error:?}"))
            .and_then(|trees| serde_json::to_value(trees).map_err(|error| error.to_string()))
            .map(without_spans)
    };
    let faithful = parse(CallMode::Faithful);
    let located = parse(CallMode::FaithfulLocated);
    match faithful == located {
        true => None,
        false => Some(format!("faithful: {faithful:?}\nlocated: {located:?}")),
    }
}

fn without_spans(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .filter(|(field, _)| field != SPAN_FIELD)
                .map(|(field, child)| (field, without_spans(child)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(without_spans).collect()),
        other => other,
    }
}
