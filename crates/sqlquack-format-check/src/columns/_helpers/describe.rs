//! DuckDB's description of a query's output columns, before and after the formatter lays it out.

use sqlquack_format::{FormatOptions, format_sql};
use sqlquack_syntax::Dialect;

/// One query at one line width: refused, or laid out with its source bound or not, and any change in its columns.
pub(crate) enum LayoutCheck {
    Refused,
    Bound(Option<String>),
    Unbound(Option<String>),
}

/// `sql` laid out for DuckDB, or the formatter's reason for refusing it.
fn duckdb_layout(sql: &str, line_width: Option<usize>) -> Result<String, String> {
    let options = FormatOptions {
        line_width,
        ..FormatOptions::new(Dialect::DuckDB)
    };
    format_sql(sql, &options)
        .map(|formatted| formatted.sql)
        .map_err(|refusal| refusal.reason)
}

/// The output columns DuckDB binds `sql` to, as names and types, or DuckDB's reason it does not bind.
fn described(connection: &duckdb::Connection, sql: &str) -> Result<Vec<(String, String)>, String> {
    let mut statement = connection
        .prepare(&format!(
            "DESCRIBE {}",
            sql.trim_end().trim_end_matches(';')
        ))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<(String, String)>, _>>()
        .map_err(|error| error.to_string())
}

/// Whether `after` binds to other columns than `before`, or binds where it does not; reasons for not binding name moved line positions, so they are not compared.
fn binds_differently(
    before: &Result<Vec<(String, String)>, String>,
    after: &Result<Vec<(String, String)>, String>,
) -> bool {
    match (before, after) {
        (Ok(before), Ok(after)) => before != after,
        (Err(_), Err(_)) => false,
        _ => true,
    }
}

/// Lay out `sql` at `line_width` and compare the columns DuckDB binds the layout to with the source's.
pub(crate) fn layout_check(
    connection: &duckdb::Connection,
    sql: &str,
    line_width: Option<usize>,
) -> LayoutCheck {
    let layout = duckdb_layout(sql, line_width);
    let before = described(connection, sql);
    match layout {
        Err(_) => LayoutCheck::Refused,
        Ok(layout) => {
            let after = described(connection, &layout);
            let change = binds_differently(&before, &after).then(|| {
                format!(
                    "{sql}\n--- laid out at {line_width:?}\n{layout}\n--- before {before:?}\n--- after {after:?}"
                )
            });
            match before {
                Ok(_) => LayoutCheck::Bound(change),
                Err(_) => LayoutCheck::Unbound(change),
            }
        }
    }
}
