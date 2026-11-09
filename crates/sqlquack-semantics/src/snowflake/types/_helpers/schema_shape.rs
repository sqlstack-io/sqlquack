//! What is wrong with a schema file's shape, in its own terms, where JSON decoding fails.

use serde_json::{Map, Value};

use crate::snowflake::types::constants::{COLUMN_KEYS, REQUIRED_COLUMN_KEYS, SCHEMA_SECTIONS};

/// The first shape problem of the schema `top`, naming the key, relation or column at fault.
pub(crate) fn shape_problem(top: &Value) -> Option<String> {
    let Value::Object(top) = top else {
        return Some(
            "the schema must be a JSON object: relations by name, or `relations` and `functions`"
                .to_owned(),
        );
    };
    let sectioned = SCHEMA_SECTIONS
        .iter()
        .any(|section| top.contains_key(*section));
    if sectioned
        && let Some(key) = top
            .keys()
            .find(|key| !SCHEMA_SECTIONS.contains(&key.as_str()))
    {
        return Some(format!(
            "unknown top-level key `{key}`; a schema with sections holds only `relations` and \
             `functions`"
        ));
    }
    let relations = match sectioned {
        true => top.get("relations")?.as_object()?,
        false => top,
    };
    relations
        .iter()
        .find_map(|(relation, columns)| relation_problem(relation, columns))
}

/// What is wrong with `relation`'s columns, if anything.
fn relation_problem(relation: &str, columns: &Value) -> Option<String> {
    let Value::Array(columns) = columns else {
        return Some(format!(
            "relation {relation} must list its columns, as [[\"NAME\", \"TYPE\"], …]"
        ));
    };
    columns.iter().enumerate().find_map(|(index, column)| {
        column_problem(
            &format!("relation {relation}, column {}", index + 1),
            column,
        )
    })
}

/// What is wrong with the column at `place`, if anything.
fn column_problem(place: &str, column: &Value) -> Option<String> {
    match column {
        Value::Array(pair) => match pair.as_slice() {
            [Value::String(_), Value::String(_)] => None,
            [Value::String(name)] => Some(format!("{place} ({name}) has no type")),
            _ => Some(format!("{place} must be [\"NAME\", \"TYPE\"], two strings")),
        },
        Value::Object(fields) => object_column_problem(place, fields),
        _ => Some(format!(
            "{place} must be [\"NAME\", \"TYPE\"] or {{\"name\": …, \"type\": …}}"
        )),
    }
}

/// What is wrong with the column at `place` written as an object, if anything.
fn object_column_problem(place: &str, fields: &Map<String, Value>) -> Option<String> {
    if let Some(key) = fields
        .keys()
        .find(|key| !COLUMN_KEYS.contains(&key.as_str()))
    {
        return Some(format!(
            "{place} has unknown key `{key}`; a column takes {}",
            COLUMN_KEYS.join(", ")
        ));
    }
    REQUIRED_COLUMN_KEYS
        .iter()
        .find(|key| !fields.get(**key).is_some_and(Value::is_string))
        .map(|key| format!("{place} needs `{key}` as a string"))
}
