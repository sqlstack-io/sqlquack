//! The query's result as text, and how it differs from the expected rows.

use duckdb::Connection;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_syntax::Expression;

use crate::run::_helpers::fixtures::insert_rows;
use crate::run::_helpers::session::duckdb_error;
use crate::run::constants::{ACTUAL, EXPECTED, JSON_TYPE, NULL_TEXT};
use crate::run::types::{Columns, Row};

/// Whether the one query in `sql` orders its rows, or why `sql` is not one query.
pub(crate) fn ordered_query(sql: &str) -> Result<bool, String> {
    let Ok(statements) = parse_source(sql, SourceDialect::Snowflake) else {
        return Ok(false);
    };
    match statements.as_slice() {
        [statement] => Ok(match statement {
            Expression::Select(query) => query.order_by.is_some(),
            Expression::Union(query) => query.order_by.is_some(),
            Expression::Intersect(query) => query.order_by.is_some(),
            Expression::Except(query) => query.order_by.is_some(),
            _ => false,
        }),
        statements => Err(format!(
            "run takes one query, and the input holds {} statements",
            statements.len()
        )),
    }
}

/// The converted query's columns, with their DuckDB types, and its rows.
pub(crate) fn query_result(
    connection: &Connection,
    sql: &str,
) -> Result<(Columns, Vec<Row>), String> {
    connection
        .execute_batch(&format!("CREATE TEMP TABLE sqlquack_actual AS {sql}"))
        .map_err(duckdb_error)?;
    let mut statement = connection
        .prepare(
            "SELECT column_name, data_type FROM duckdb_columns() \
             WHERE database_name = 'temp' AND table_name = 'sqlquack_actual' ORDER BY column_index",
        )
        .map_err(duckdb_error)?;
    let columns: Columns = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .and_then(Iterator::collect)
        .map_err(duckdb_error)?;
    let rows = rendered(connection, ACTUAL, &columns)?;
    Ok((columns, rows))
}

/// `expect`'s rows, typed as the result's columns and rendered as its rows are.
pub(crate) fn expected_rows(
    connection: &Connection,
    columns: &[(String, String)],
    expect: &str,
) -> Result<Vec<Row>, String> {
    connection
        .execute_batch(&format!(
            "CREATE TEMP TABLE sqlquack_expected AS SELECT * FROM {ACTUAL} LIMIT 0"
        ))
        .map_err(duckdb_error)?;
    let names: Vec<String> = columns.iter().map(|(name, _)| name.clone()).collect();
    insert_rows(connection, (EXPECTED, &names), expect, "--expect")?;
    rendered(connection, EXPECTED, columns)
}

/// How `actual` differs from `expected`, in order when the query orders its rows.
pub(crate) fn differences(expected: Vec<Row>, actual: Vec<Row>, ordered: bool) -> Vec<String> {
    match ordered {
        true => ordered_differences(&expected, &actual),
        false => bag_differences(expected, actual),
    }
}

/// The result as a plain table: a header, then one line per row, columns padded to line up.
pub(crate) fn result_table(names: &[String], rows: &[Row]) -> String {
    let cells: Vec<Vec<&str>> = std::iter::once(names.iter().map(String::as_str).collect())
        .chain(rows.iter().map(row_cells))
        .collect();
    let widths: Vec<usize> = (0..names.len())
        .map(|column| column_width(&cells, column))
        .collect();
    cells
        .iter()
        .map(|line| padded_line(line, &widths))
        .collect()
}

/// The rows of `table`, each value as text.
fn rendered(
    connection: &Connection,
    table: &str,
    columns: &[(String, String)],
) -> Result<Vec<Row>, String> {
    if columns.is_empty() {
        return Ok(Vec::new());
    }
    let log = ProvenanceLog::default();
    let values: Vec<String> = columns
        .iter()
        .map(|(name, data_type)| {
            let name = quoted_identifier(name, &log);
            match data_type.as_str() {
                JSON_TYPE => format!("CAST(json({name}) AS VARCHAR)"),
                _ => format!("CAST({name} AS VARCHAR)"),
            }
        })
        .collect();
    let mut statement = connection
        .prepare(&format!("SELECT {} FROM {table}", values.join(", ")))
        .map_err(duckdb_error)?;
    let count = columns.len();
    statement
        .query_map([], |row| (0..count).map(|index| row.get(index)).collect())
        .and_then(Iterator::collect)
        .map_err(duckdb_error)
}

/// The first row where ordered rows differ, or how their counts differ.
fn ordered_differences(expected: &[Row], actual: &[Row]) -> Vec<String> {
    if expected.len() != actual.len() {
        return vec![format!(
            "expected {} rows, got {}",
            expected.len(),
            actual.len()
        )];
    }
    match expected
        .iter()
        .zip(actual)
        .position(|(wanted, got)| wanted != got)
    {
        Some(row) => vec![
            format!("row {}: expected {}", row + 1, shown(&expected[row])),
            format!("row {}: got      {}", row + 1, shown(&actual[row])),
        ],
        None => Vec::new(),
    }
}

/// The expected rows the result lacks, then the rows it holds beyond them, in any order.
fn bag_differences(mut expected: Vec<Row>, mut actual: Vec<Row>) -> Vec<String> {
    expected.sort();
    actual.sort();
    let mut missing: Vec<Row> = Vec::new();
    let mut unexpected: Vec<Row> = Vec::new();
    let (mut left, mut right) = (
        expected.into_iter().peekable(),
        actual.into_iter().peekable(),
    );
    loop {
        match (left.peek(), right.peek()) {
            (None, None) => break,
            (Some(wanted), Some(got)) if wanted == got => {
                left.next();
                right.next();
            }
            (Some(wanted), Some(got)) if wanted < got => missing.extend(left.next()),
            (Some(_), None) => missing.extend(left.next()),
            _ => unexpected.extend(right.next()),
        }
    }
    missing
        .iter()
        .map(|row| format!("missing    {}", shown(row)))
        .chain(
            unexpected
                .iter()
                .map(|row| format!("unexpected {}", shown(row))),
        )
        .collect()
}

/// A row as `(1, a, NULL)`.
fn shown(row: &Row) -> String {
    format!("({})", row_cells(row).join(", "))
}

/// A row's values as text, NULL spelled out.
fn row_cells(row: &Row) -> Vec<&str> {
    row.iter()
        .map(|value| value.as_deref().unwrap_or(NULL_TEXT))
        .collect()
}

/// The widest cell of `column`.
fn column_width(cells: &[Vec<&str>], column: usize) -> usize {
    cells
        .iter()
        .map(|line| line[column].chars().count())
        .max()
        .unwrap_or(0)
}

/// One table line, each cell padded to its column's width.
fn padded_line(line: &[&str], widths: &[usize]) -> String {
    let padded: Vec<String> = line
        .iter()
        .zip(widths)
        .map(|(cell, width)| format!("{cell:<width$}"))
        .collect();
    padded.join("  ").trim_end().to_owned() + "\n"
}
