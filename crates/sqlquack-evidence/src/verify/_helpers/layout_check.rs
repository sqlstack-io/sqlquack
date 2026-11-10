//! The layout check: the laid-out SQL returns exactly what the single-line SQL returns.

use duckdb::Connection;
use serde_json::Value;
use sqlquack_compare::values::models::ResultSet;

use crate::replay::main::replay_on_engine::replay_on_engine;
use crate::replay::models::{PythonReplay, ReplaySettings};

/// The divergence whose values read the clock, so two replays may differ by the time between them.
const CLOCK_DIVERGENCE: &str = "current_time";

/// The fields of DuckDB's parse tree that hold where a node starts in the SQL text.
const POSITION_FIELDS: [&str; 2] = ["query_location", "query_location_length"];

/// Why the laid-out SQL returns another result or error than the single-line `sql`, if it does.
pub(crate) fn layout_failure(
    laid_out: &Result<String, String>,
    (setup, sql): (&[String], &str),
    (single_line, reads_clock): (&Result<ResultSet, String>, bool),
    (python, settings): (Option<&PythonReplay>, &ReplaySettings),
) -> Option<String> {
    let laid_out = match laid_out {
        Ok(laid_out) if laid_out == sql => return None,
        Ok(laid_out) if parsed_alike(sql, laid_out) => return None,
        Ok(laid_out) => laid_out,
        Err(reason) => return Some(format!("the layout changed more than whitespace: {reason}")),
    };
    let replayed = replay_on_engine((setup, laid_out), python, settings);
    let same = match (single_line, &replayed) {
        (Ok(single_line), Ok(replayed)) => same_result(single_line, replayed, reads_clock),
        (Err(single_line), Err(replayed)) => first_line(single_line) == first_line(replayed),
        _ => false,
    };
    (!same).then(|| {
        format!(
            "the laid-out SQL returned another result than the single-line SQL: {}",
            match &replayed {
                Ok(_) => "different rows or columns".to_owned(),
                Err(error) => first_line(error).to_owned(),
            }
        )
    })
}

/// Whether the provenance's divergences include one whose values read the clock.
pub(crate) fn reads_clock<'name>(mut divergences: impl Iterator<Item = &'name String>) -> bool {
    divergences.any(|divergence| divergence == CLOCK_DIVERGENCE)
}

/// Whether two results have the same schema and, unless they read the clock, the same rows.
fn same_result(left: &ResultSet, right: &ResultSet, reads_clock: bool) -> bool {
    left.schema == right.schema && (reads_clock || left.batches == right.batches)
}

/// An error's first line; later lines quote the SQL with its line numbers.
fn first_line(error: &str) -> &str {
    error.lines().next().unwrap_or_default()
}

/// Whether DuckDB parses both texts into the same statements, apart from source positions.
fn parsed_alike(left: &str, right: &str) -> bool {
    let Ok(connection) = Connection::open_in_memory() else {
        return false;
    };
    match (
        parse_tree(&connection, left),
        parse_tree(&connection, right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// DuckDB's parse tree of `sql` without source positions, or why DuckDB cannot serialize it.
fn parse_tree(connection: &Connection, sql: &str) -> Result<Value, String> {
    let text: String = connection
        .query_row("SELECT json_serialize_sql(?)", [sql], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let mut tree: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if tree.get("error") != Some(&Value::Bool(false)) {
        return Err(text);
    }
    drop_positions(&mut tree);
    Ok(tree)
}

/// Removes every source position field from a parse tree, at every depth.
fn drop_positions(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.retain(|name, _| !POSITION_FIELDS.contains(&name.as_str()));
            fields.values_mut().for_each(drop_positions);
        }
        Value::Array(items) => items.iter_mut().for_each(drop_positions),
        _ => {}
    }
}
