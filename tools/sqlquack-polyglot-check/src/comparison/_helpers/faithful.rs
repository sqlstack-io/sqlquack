//! Faithful calls put back into polyglot's lowered form, one call at a time.

use serde_json::{Map, Value};
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::comparison::constants::{
    AGGREGATE_KEY, ANNOTATED_KEY, COMMENTS_SUFFIX, ERROR_KIND, FUNCTION_KEY, JSON_EXTRACT_KEY,
    SELECT_PREFIX,
};
use crate::comparison::models::Spelling;

/// The tree with each lowered faithful call replaced by polyglot's parse of its text, and why.
pub(crate) fn relower(value: Value, sql: &[char]) -> (Value, Vec<Spelling>) {
    let mut spellings: Vec<Spelling> = Vec::new();
    let value = match call_text(&value, sql) {
        Some((name, start, text)) => {
            let value = without_span(value);
            match lowered_expression(&text, start, sql) {
                Some(lowered) if same_call(&value, &lowered) => value,
                Some(lowered) => {
                    spellings.push(Spelling {
                        name,
                        polyglot: node_kind(&lowered),
                    });
                    return (lowered, spellings);
                }
                None => {
                    spellings.push(Spelling {
                        name,
                        polyglot: ERROR_KIND.to_owned(),
                    });
                    value
                }
            }
        }
        None => value,
    };
    let value = match value {
        Value::Object(object) => {
            let mut children: Map<String, Value> = Map::new();
            for (key, child) in object {
                let (child, found) = relower(child, sql);
                spellings.extend(found);
                children.insert(key, child);
            }
            Value::Object(children)
        }
        Value::Array(items) => {
            let mut children: Vec<Value> = Vec::with_capacity(items.len());
            for child in items {
                let (child, found) = relower(child, sql);
                spellings.extend(found);
                children.push(child);
            }
            Value::Array(children)
        }
        other => other,
    };
    (value, spellings)
}

/// The upper-case name, start offset and source text of a faithful call with a recorded span.
fn call_text(value: &Value, sql: &[char]) -> Option<(String, usize, String)> {
    let payload = call_payload(value)?;
    let span = payload.get("span")?;
    let start = offset(span.get("start")?)?;
    let end = offset(span.get("end")?)?;
    let name = payload.get("name")?.as_str()?.to_ascii_uppercase();
    Some((name, start, sql.get(start..end)?.iter().collect()))
}

/// A serialised character offset; one past any text when it does not fit.
fn offset(value: &Value) -> Option<usize> {
    let number = value.as_u64()?;
    Some(usize::try_from(number).unwrap_or(usize::MAX))
}

/// The function or aggregate payload of a call node.
fn call_payload(value: &Value) -> Option<&Value> {
    let object = value.as_object()?;
    object
        .get(FUNCTION_KEY)
        .or_else(|| object.get(AGGREGATE_KEY))
}

/// The call node without its recorded span.
fn without_span(mut value: Value) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };
    let key = match object.contains_key(FUNCTION_KEY) {
        true => FUNCTION_KEY,
        false => AGGREGATE_KEY,
    };
    if let Some(Value::Object(payload)) = object.get_mut(key) {
        payload.remove("span");
    }
    value
}

/// polyglot's lowered parse of `text`, found at `start` in `sql`, with spans placed in `sql`.
fn lowered_expression(text: &str, start: usize, sql: &[char]) -> Option<Value> {
    let prefixed = format!("{SELECT_PREFIX}{text}");
    let parsed = parse_statements_with(
        &prefixed,
        Dialect::Snowflake,
        &ParseOptions::default(),
        CallMode::Lowered,
    );
    let Ok(statements) = parsed else {
        return None;
    };
    let Ok(statement) = serde_json::to_value(statements.first()?) else {
        return None;
    };
    let expression = statement.get("select")?.get("expressions")?.get(0)?.clone();
    Some(moved_spans(expression, start, sql))
}

/// The tree with every span moved from the `SELECT `-prefixed text to the call's place in `sql`.
fn moved_spans(value: Value, start: usize, sql: &[char]) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, child)| match (key.as_str(), child) {
                    ("span", Value::Object(span)) => {
                        (key, Value::Object(moved_span(span, start, sql)))
                    }
                    (_, child) => (key, moved_spans(child, start, sql)),
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|child| moved_spans(child, start, sql))
                .collect(),
        ),
        other => other,
    }
}

/// One span with shifted offsets and the line and column after its last character.
fn moved_span(mut span: Map<String, Value>, start: usize, sql: &[char]) -> Map<String, Value> {
    let read = |key: &str| span.get(key).and_then(offset);
    let (Some(from), Some(to)) = (read("start"), read("end")) else {
        return span;
    };
    let from = from + start - SELECT_PREFIX.len();
    let to = to + start - SELECT_PREFIX.len();
    let before = &sql[..to.min(sql.len())];
    let line_start = before
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |index| index + 1);
    let line = 1 + before
        .iter()
        .filter(|character| **character == '\n')
        .count();
    span.insert("start".to_owned(), from.into());
    span.insert("end".to_owned(), to.into());
    span.insert("line".to_owned(), line.into());
    span.insert("column".to_owned(), (to - line_start + 1).into());
    span
}

/// Whether `lowered` is the same kind of call node, with the same name and argument count.
fn same_call(faithful: &Value, lowered: &Value) -> bool {
    let (Some(ours), Some(theirs)) = (call_payload(faithful), call_payload(lowered)) else {
        return false;
    };
    let name = |payload: &Value| {
        payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_ascii_uppercase)
    };
    let arity = |payload: &Value| payload.get("args").and_then(Value::as_array).map(Vec::len);
    node_kind(faithful) == node_kind(lowered)
        && name(ours) == name(theirs)
        && arity(ours) == arity(theirs)
}

/// The node's serialised variant name, such as `length` or `if_func`.
pub(crate) fn node_kind(value: &Value) -> String {
    value
        .as_object()
        .and_then(|object| object.keys().next())
        .cloned()
        .unwrap_or_default()
}

/// The tree with every comment removed, and annotations that hold only comments unwrapped.
pub(crate) fn without_comments(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            if let Some(Value::Object(annotated)) = object.get(ANNOTATED_KEY)
                && annotated.get("type_assertion").is_none_or(Value::is_null)
                && annotated.get("type_like").is_none_or(Value::is_null)
                && let Some(this) = annotated.get("this")
            {
                return without_comments(this);
            }
            Value::Object(
                object
                    .iter()
                    .filter(|(key, _)| !key.ends_with(COMMENTS_SUFFIX))
                    .map(|(key, child)| (key.clone(), without_comments(child)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(without_comments).collect()),
        other => other.clone(),
    }
}

/// Every comment in the tree, in order.
pub(crate) fn comments(value: &Value) -> Vec<String> {
    match value {
        Value::Object(object) => object
            .iter()
            .flat_map(
                |(key, child)| match (key.ends_with(COMMENTS_SUFFIX), child) {
                    (true, Value::Array(items)) => items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                    _ => comments(child),
                },
            )
            .collect(),
        Value::Array(items) => items.iter().flat_map(comments).collect(),
        _ => Vec::new(),
    }
}

/// The tree with variant paths flat: the faithful `a["x.y"]` and polyglot's `a.x.y` agree.
pub(crate) fn with_flat_paths(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, child)| match (key.as_str(), child) {
                    (JSON_EXTRACT_KEY, Value::Object(extract)) => {
                        (key.clone(), Value::Object(flat_extract(extract)))
                    }
                    _ => (key.clone(), with_flat_paths(child)),
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(with_flat_paths).collect()),
        other => other.clone(),
    }
}

fn flat_extract(extract: &Map<String, Value>) -> Map<String, Value> {
    let mut flat: Map<String, Value> = extract
        .iter()
        .map(|(key, child)| (key.clone(), with_flat_paths(child)))
        .collect();
    if let Some(Value::String(path)) = flat
        .get_mut("expression")
        .and_then(|expression| expression.get_mut("literal"))
        .and_then(|literal| literal.get_mut("value"))
    {
        *path = flat_path(path);
    }
    flat
}

/// `["k"]` segments written as `.k`, without a leading dot.
fn flat_path(path: &str) -> String {
    let mut flat = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(start) = rest.find("[\"") {
        flat.push_str(&rest[..start]);
        let Some(end) = rest[start + 2..].find("\"]") else {
            break;
        };
        if !flat.is_empty() {
            flat.push('.');
        }
        flat.push_str(&rest[start + 2..start + 2 + end].replace("\\\"", "\""));
        rest = &rest[start + 2 + end + 2..];
    }
    flat.push_str(rest);
    flat
}
