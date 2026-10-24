//! Rewrites a serialised tree to the spelling-free form both sides are compared in.

use polyglot_sql::Expression;
use serde_json::{Map, Value};

use crate::comparison::constants::SPELLING_ONLY_FIELDS;

const ANNOTATED: &str = "annotated";
const LITERAL_TYPE: &str = "literal_type";
const DOLLAR_STRING: &str = "dollar_string";
const STRING: &str = "string";

pub(crate) fn normalise(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalise).collect()),
        Value::Object(object) => normalise_object(object),
        other => other,
    }
}

fn normalise_object(object: Map<String, Value>) -> Value {
    if object.len() == 1
        && let Some(Value::Object(annotated)) = object.get(ANNOTATED)
        && let Some(inner) = annotated.get("this")
    {
        return normalise(inner.clone());
    }
    let mut normalised = Map::new();
    let function_like =
        object.contains_key("args") && !matches!(object.get("quoted"), Some(Value::Bool(true)));
    for (key, value) in object {
        if SPELLING_ONLY_FIELDS.iter().any(|field| field.matches(&key)) {
            continue;
        }
        let value = match (key.as_str(), value) {
            ("name", Value::String(name)) if function_like => {
                Value::String(name.to_ascii_uppercase())
            }
            (LITERAL_TYPE, Value::String(kind)) if kind == DOLLAR_STRING => {
                Value::String(STRING.to_owned())
            }
            (_, value) => normalise(value),
        };
        normalised.insert(key, value);
    }
    Value::Object(normalised)
}

pub(crate) fn normalised_tree(statement: &Expression) -> Result<Value, serde_json::Error> {
    serde_json::to_value(statement).map(normalise)
}
