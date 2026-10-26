//! `compare_recordings`: a stored Snowflake result against a fresh recording of the same case.

use crate::comparison::_helpers::rows::{RowRules, compare_rows};
use crate::comparison::models::{Mismatch, MismatchKind, RecordedResult, RowOrder};

/// Requires identical names, identical Snowflake types and exactly equal values.
pub fn compare_recordings(
    stored: &RecordedResult,
    fresh: &RecordedResult,
    order: RowOrder,
) -> Result<(), Mismatch> {
    if stored.columns != fresh.columns {
        return Err(Mismatch {
            kind: MismatchKind::ColumnType,
            detail: format!(
                "stored columns {} differ from fresh columns {}",
                describe(stored),
                describe(fresh)
            ),
        });
    }
    compare_rows(
        &stored.rows,
        &fresh.rows,
        &RowRules {
            order,
            float32_ulps: 0,
            float64_ulps: 0,
            ignore_zone_labels: false,
            json_columns: Vec::new(),
        },
    )
}

fn describe(result: &RecordedResult) -> String {
    result
        .columns
        .iter()
        .map(|column| format!("{} {}", column.name, column.snowflake_type))
        .collect::<Vec<_>>()
        .join(", ")
}
