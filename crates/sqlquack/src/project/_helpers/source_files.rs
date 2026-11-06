//! Tables declared in SQLBuild's source YAML files: a name and columns of a name and a type each.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::project::_helpers::yaml_value::yaml_value;

/// One source file; keys SQLQuack does not read, such as contracts and audits, are ignored.
#[derive(Deserialize)]
struct SourceFile {
    #[serde(default)]
    sources: Vec<SourceEntry>,
}

#[derive(Deserialize)]
struct SourceEntry {
    name: String,
    #[serde(default)]
    columns: Vec<SourceColumn>,
    /// An inline query rather than a table; its columns may be partial, so it is not declared.
    #[serde(default)]
    expression: Option<Value>,
}

#[derive(Deserialize)]
struct SourceColumn {
    name: String,
    #[serde(rename = "type", default)]
    type_name: Option<Value>,
}

/// Each table `path` declares, upper-cased as Snowflake folds SQLBuild's unquoted names.
pub(crate) fn source_relations(path: &Path) -> Result<Vec<(String, Value)>, String> {
    let failed = |reason: String| format!("{}: {reason}", path.display());
    let text = std::fs::read_to_string(path).map_err(|error| failed(error.to_string()))?;
    let value = yaml_value(&text).map_err(failed)?;
    if value.is_null() {
        return Ok(Vec::new());
    }
    let file: SourceFile = serde_json::from_value(value)
        .map_err(|error| failed(format!("is not a SQLBuild source file: {error}")))?;
    Ok(file.sources.into_iter().filter_map(table).collect())
}

/// A source as a table and its `[column, type]` pairs; None for an inline query or an untyped column.
fn table(source: SourceEntry) -> Option<(String, Value)> {
    if source.expression.is_some() || source.columns.is_empty() {
        return None;
    }
    let columns: Option<Vec<Value>> = source.columns.into_iter().map(typed_column).collect();
    Some((source.name.to_ascii_uppercase(), Value::Array(columns?)))
}

/// A column as an upper-case name and its type text, or None without a text type.
fn typed_column(column: SourceColumn) -> Option<Value> {
    match column.type_name {
        Some(Value::String(type_name)) => {
            Some(json!([column.name.to_ascii_uppercase(), type_name]))
        }
        _ => None,
    }
}
