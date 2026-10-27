use serde_json::Value;
use sqlquack_syntax::{parse_statements_with, CallMode, Dialect, ParseOptions};

pub(crate) fn statements(sql: &str, dialect: Dialect, mode: CallMode) -> Value {
    serde_json::to_value(
        parse_statements_with(sql, dialect, &ParseOptions::default(), mode)
            .expect("the SQL parses"),
    )
    .expect("trees serialise")
}

pub(crate) fn first_projection(sql: &str, mode: CallMode) -> Value {
    statements(sql, Dialect::Snowflake, mode)[0]["select"]["expressions"][0].clone()
}

pub(crate) fn kind(value: &Value) -> String {
    value
        .as_object()
        .and_then(|object| object.keys().next())
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn faithful_call(sql: &str) -> (Value, Option<usize>, String) {
    let faithful = first_projection(sql, CallMode::Faithful);
    let payload = faithful
        .get("function")
        .or_else(|| faithful.get("aggregate_function"))
        .cloned()
        .unwrap_or_default();
    (
        payload["name"].clone(),
        payload["args"].as_array().map(Vec::len),
        kind(&first_projection(sql, CallMode::Lowered)),
    )
}

pub(crate) fn same_in_both_modes(sql: &str, dialect: Dialect) -> bool {
    statements(sql, dialect, CallMode::Faithful) == statements(sql, dialect, CallMode::Lowered)
}

pub(crate) fn projection_at(sql: &str, pointer: &str) -> Value {
    first_projection(sql, CallMode::Faithful)
        .pointer(pointer)
        .cloned()
        .unwrap_or_default()
}
