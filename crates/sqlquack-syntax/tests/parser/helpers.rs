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

/// The source text a located parse records for the node at `pointer`, or why there is none.
pub(crate) fn located_text(sql: &str, pointer: &str) -> Result<String, String> {
    let tree = statements(sql, Dialect::Snowflake, CallMode::FaithfulLocated);
    let node = tree.pointer(pointer).ok_or("no node at the pointer")?;
    let span = node
        .as_object()
        .and_then(|object| object.values().next())
        .and_then(|payload| payload.get("span"))
        .or_else(|| node.get("span"))
        .ok_or("the node has no span")?;
    let start = span["start"].as_u64().ok_or("no start")? as usize;
    let end = span["end"].as_u64().ok_or("no end")? as usize;
    Ok(sql.chars().skip(start).take(end - start).collect())
}

/// Whether a located parse equals the faithful parse, spans aside.
pub(crate) fn located_matches_faithful(sql: &str, dialect: Dialect) -> bool {
    let parse = |mode| parse_statements_with(sql, dialect, &ParseOptions::default(), mode).ok();
    parse(CallMode::FaithfulLocated) == parse(CallMode::Faithful)
}

/// The cast target types of `SELECT x::<type>` and `SELECT CAST(x AS <type>)`.
pub(crate) fn cast_targets(type_text: &str) -> (Value, Value) {
    let target = |sql: String| first_projection(&sql, CallMode::Faithful)["cast"]["to"].clone();
    (
        target(format!("SELECT x::{type_text}")),
        target(format!("SELECT CAST(x AS {type_text})")),
    )
}

pub(crate) fn accepted(sql: &str, dialect: Dialect) -> bool {
    parse_statements_with(sql, dialect, &ParseOptions::default(), CallMode::Faithful).is_ok()
}
