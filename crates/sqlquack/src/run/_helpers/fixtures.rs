//! The project's tables, created in DuckDB and filled from `--fixture` rows or files.

use std::path::Path;

use duckdb::Connection;
use serde_json::Value;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::checking::main::create_table::create_table;
use sqlquack_blocks::checking::main::quoted_relation::quoted_relation;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::types::models::Schema;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::run::_helpers::session::{duckdb_error, literal};

/// Creates every table the project declares, and fills those `fixtures` name.
pub(crate) fn load_fixtures(
    connection: &Connection,
    schema: &Schema,
    fixtures: &[String],
) -> Result<(), String> {
    for (name, columns) in &schema.relations {
        let typed: Vec<(String, SnowflakeType)> = columns
            .iter()
            .map(|column| (column.name.clone(), column.data_type))
            .collect();
        connection
            .execute_batch(&create_table(name, &typed))
            .map_err(duckdb_error)?;
    }
    let tables: Vec<&String> = schema.relations.keys().collect();
    for fixture in fixtures {
        let Some((table, rows)) = fixture.split_once('=') else {
            return Err(format!("--fixture {fixture} is not TABLE=ROWS"));
        };
        let relation = declared_table(table.trim(), &tables)?;
        let names: Vec<String> = schema.relations[relation]
            .iter()
            .map(|column| column.name.clone())
            .collect();
        let target = quoted_relation(relation, &ProvenanceLog::default());
        let rows = rows.trim();
        match (rows.starts_with('['), rows.contains('/')) {
            (true, _) => insert_rows(connection, (&target, &names), rows, table)?,
            (false, true) => insert_file(connection, (&target, &names), rows)?,
            (false, false) => {
                return Err(format!(
                    "--fixture {table}= takes rows as a JSON list, or a path holding a /, \
                     such as ./{}.csv",
                    table.to_ascii_lowercase()
                ));
            }
        }
    }
    Ok(())
}

/// Inserts the JSON list `rows` into `target`: lists in column order, or objects naming columns.
pub(crate) fn insert_rows(
    connection: &Connection,
    (target, columns): (&str, &[String]),
    rows: &str,
    label: &str,
) -> Result<(), String> {
    let rows: Vec<Value> = serde_json::from_str(rows)
        .map_err(|error| format!("the rows of {label} are not a JSON list of rows: {error}"))?;
    let log = ProvenanceLog::default();
    for row in rows {
        let (positions, values) = row_values(row, columns, label)?;
        if positions.is_empty() {
            connection
                .execute_batch(&format!("INSERT INTO {target} DEFAULT VALUES"))
                .map_err(duckdb_error)?;
            continue;
        }
        let names: Vec<String> = positions
            .iter()
            .map(|&position| quoted_identifier(&columns[position], &log))
            .collect();
        let values: Vec<String> = values.iter().map(sql_value).collect();
        connection
            .execute_batch(&format!(
                "INSERT INTO {target} ({}) VALUES ({})",
                names.join(", "),
                values.join(", ")
            ))
            .map_err(duckdb_error)?;
    }
    Ok(())
}

/// A row's values with the positions of the columns they fill; an object's missing ones stay NULL.
fn row_values(
    row: Value,
    columns: &[String],
    label: &str,
) -> Result<(Vec<usize>, Vec<Value>), String> {
    match row {
        Value::Array(values) if values.len() == columns.len() => {
            Ok(((0..values.len()).collect(), values))
        }
        Value::Array(values) => Err(format!(
            "a row of {label} has {} values, and there are {} columns",
            values.len(),
            columns.len()
        )),
        Value::Object(object) => {
            let mut pairs: Vec<(usize, Value)> = Vec::new();
            for (key, value) in object {
                pairs.push((column_position(&key, columns)?, value));
            }
            Ok(pairs.into_iter().unzip())
        }
        other => Err(format!(
            "a row of {label} is {other}, not a list or an object"
        )),
    }
}

/// Inserts a CSV file with a header, or a Parquet file, into `target`, its columns matched by name.
fn insert_file(
    connection: &Connection,
    (target, columns): (&str, &[String]),
    path: &str,
) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!("cannot read {path}"));
    }
    let lower = path.to_ascii_lowercase();
    let reader = match () {
        _ if lower.ends_with(".csv") => format!(
            "read_csv({}, header = true, all_varchar = true)",
            literal(path)
        ),
        _ if lower.ends_with(".parquet") => format!("read_parquet({})", literal(path)),
        _ => return Err(format!("{path} is neither a .csv nor a .parquet file")),
    };
    let mut statement = connection
        .prepare(&format!(
            "SELECT column_name FROM (DESCRIBE SELECT * FROM {reader})"
        ))
        .map_err(duckdb_error)?;
    let found: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .and_then(Iterator::collect)
        .map_err(duckdb_error)?;
    let log = ProvenanceLog::default();
    let mut targets: Vec<String> = Vec::new();
    for name in &found {
        let position =
            column_position(name, columns).map_err(|error| format!("{path}: {error}"))?;
        targets.push(quoted_identifier(&columns[position], &log));
    }
    let sources: Vec<String> = found
        .iter()
        .map(|name| quoted_identifier(name, &log))
        .collect();
    connection
        .execute_batch(&format!(
            "INSERT INTO {target} ({}) SELECT {} FROM {reader}",
            targets.join(", "),
            sources.join(", ")
        ))
        .map_err(duckdb_error)
}

/// The declared table `name` means: its whole name or its last parts, ignoring case.
fn declared_table<'a>(name: &str, tables: &[&'a String]) -> Result<&'a String, String> {
    let wanted = name.to_ascii_uppercase();
    let found: Vec<&String> = tables
        .iter()
        .copied()
        .filter(|table| {
            let table = table.to_ascii_uppercase();
            table == wanted || table.ends_with(&format!(".{wanted}"))
        })
        .collect();
    match found.as_slice() {
        [table] => Ok(table),
        [] => Err(format!(
            "no source of the project declares the table {name}"
        )),
        _ => Err(format!(
            "{name} could be {}; give the whole name",
            joined(&found)
        )),
    }
}

/// The column `name` means, folded as Snowflake folds names; else the one matching ignoring case.
fn column_position(name: &str, columns: &[String]) -> Result<usize, String> {
    let folded = match name
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
    {
        Some(quoted) => quoted.replace("\"\"", "\""),
        None => name.to_ascii_uppercase(),
    };
    if let Some(position) = columns.iter().position(|column| *column == folded) {
        return Ok(position);
    }
    let found: Vec<usize> = (0..columns.len())
        .filter(|&position| columns[position].eq_ignore_ascii_case(&folded))
        .collect();
    match found.as_slice() {
        [position] => Ok(*position),
        [] => Err(format!(
            "there is no column {name}, only {}",
            joined(&columns.iter().collect::<Vec<&String>>())
        )),
        _ => Err(format!(
            "the column {name} could be any of several; give its exact case"
        )),
    }
}

/// A JSON value as the SQL literal DuckDB casts into the column: text for lists and objects.
fn sql_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => literal(text),
        other => literal(&other.to_string()),
    }
}

/// Names joined for a message.
fn joined(names: &[&String]) -> String {
    names
        .iter()
        .map(|name| name.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}
