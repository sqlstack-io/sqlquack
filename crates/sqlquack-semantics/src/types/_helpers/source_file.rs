//! Tables declared in a source YAML file: a name and columns of a name and a type each.

use serde::Deserialize;
use serde_json::{Map, Value, json};
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::main::cast_target::cast_target;

use crate::types::_helpers::yaml_value::yaml_value;
use crate::types::models::SourceTables;

/// One source file; keys SQLQuack does not read, such as contracts and audits, are ignored.
#[derive(Deserialize)]
struct SourceFile {
    #[serde(default)]
    sources: Vec<SourceEntry>,
}

#[derive(Deserialize)]
struct SourceEntry {
    name: String,
    /// The table's database, schema and name where they differ from the source's name.
    #[serde(default)]
    database: Option<String>,
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    table: Option<String>,
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
    /// The offset every value of a TIMESTAMP_TZ column holds: `+00:00` or `UTC`.
    #[serde(default)]
    offset: Option<String>,
    /// Whether a VARIANT column holds typed values, such as dates, beside JSON.
    #[serde(default)]
    typed_leaves: bool,
}

/// Each table `text` declares, its names folded as Snowflake folds them, and warnings.
pub(crate) fn source_relations(text: &str) -> Result<SourceTables, String> {
    let value = yaml_value(text)?;
    if value.is_null() {
        return Ok(SourceTables::default());
    }
    let file: SourceFile =
        serde_json::from_value(value).map_err(|error| format!("is not a source file: {error}"))?;
    let mut found = SourceTables::default();
    for source in file.sources {
        match left_out(&source) {
            Some(reason) => found.warnings.push(format!(
                "the source {} is left out, because {reason}",
                source.name
            )),
            None => found.tables.extend(table(source)),
        }
    }
    Ok(found)
}

/// `name` as Snowflake resolves it: upper case, or as written inside double quotes.
fn folded_name(name: &str) -> String {
    match name
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
    {
        Some(quoted) => quoted.replace("\"\"", "\""),
        None => name.to_ascii_uppercase(),
    }
}

/// Why a table source is left out: it lists no columns, or a column has no type SQLQuack reads.
fn left_out(source: &SourceEntry) -> Option<String> {
    if source.expression.is_some() {
        return None;
    }
    if source.columns.is_empty() {
        return Some("it lists no columns".to_owned());
    }
    let log = ProvenanceLog::default();
    source
        .columns
        .iter()
        .find_map(|column| untyped(column, &log))
}

/// Why `column` has no type SQLQuack reads: none is given, or the one given does not parse.
fn untyped(column: &SourceColumn, log: &ProvenanceLog) -> Option<String> {
    let Some(Value::String(type_name)) = &column.type_name else {
        return Some(format!("its column {} has no type", column.name));
    };
    let error = cast_target(type_name, log).err()?;
    Some(format!(
        "its column {} has no type SQLQuack reads: {error}",
        column.name
    ))
}

/// A source as a table name and its columns; None for an inline query or an untyped column.
fn table(source: SourceEntry) -> Option<(String, Value)> {
    if source.expression.is_some() || source.columns.is_empty() {
        return None;
    }
    let name = [
        source.database.as_deref(),
        source.schema.as_deref(),
        Some(source.table.as_deref().unwrap_or(&source.name)),
    ]
    .into_iter()
    .flatten()
    .map(folded_name)
    .collect::<Vec<String>>()
    .join(".");
    let columns: Option<Vec<Value>> = source.columns.into_iter().map(typed_column).collect();
    Some((name, Value::Array(columns?)))
}

/// A column as its folded name and type, with its offset and typed leaves when it has them.
fn typed_column(column: SourceColumn) -> Option<Value> {
    let Some(Value::String(type_name)) = column.type_name else {
        return None;
    };
    let name = folded_name(&column.name);
    if column.offset.is_none() && !column.typed_leaves {
        return Some(json!([name, type_name]));
    }
    let mut described = Map::new();
    described.insert("name".to_owned(), json!(name));
    described.insert("type".to_owned(), json!(type_name));
    if let Some(offset) = column.offset {
        described.insert("offset".to_owned(), json!(offset));
    }
    if column.typed_leaves {
        described.insert("typed_leaves".to_owned(), json!(true));
    }
    Some(Value::Object(described))
}
