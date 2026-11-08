//! `compare_columns`: bind each query and its DuckDB layouts in DuckDB and compare their output columns.

use crate::columns::_helpers::describe::{LayoutCheck, layout_check};
use crate::columns::constants::CORPUS_LINE_WIDTH;
use crate::columns::models::ColumnsOutcome;

/// Lay out every query for DuckDB at no width and the corpus width; report layouts whose output columns differ from the source's.
pub fn compare_columns(connection: &duckdb::Connection, queries: &[&str]) -> ColumnsOutcome {
    let checks: Vec<LayoutCheck> = queries
        .iter()
        .flat_map(|sql| [(*sql, None), (*sql, Some(CORPUS_LINE_WIDTH))])
        .map(|(sql, line_width)| layout_check(connection, sql, line_width))
        .collect();
    ColumnsOutcome {
        bound: checks
            .iter()
            .filter(|check| matches!(check, LayoutCheck::Bound(_)))
            .count(),
        unbound: checks
            .iter()
            .filter(|check| matches!(check, LayoutCheck::Unbound(_)))
            .count(),
        refused: checks
            .iter()
            .filter(|check| matches!(check, LayoutCheck::Refused))
            .count(),
        changes: checks
            .into_iter()
            .filter_map(|check| match check {
                LayoutCheck::Bound(change) | LayoutCheck::Unbound(change) => change,
                LayoutCheck::Refused => None,
            })
            .collect(),
    }
}
