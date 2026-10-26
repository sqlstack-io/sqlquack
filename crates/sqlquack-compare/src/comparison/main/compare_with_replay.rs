//! `compare_with_replay`: a recorded Snowflake result against the DuckDB replay.

use sqlquack_policy::policy::main::matching_type_equality::matching_type_equality;
use sqlquack_policy::policy::models::{
    ColumnNameMatching, ComparatorPolicy, TimestampTzComparison,
};

use crate::comparison::_helpers::rows::{RowRules, compare_rows};
use crate::comparison::_helpers::type_names::duckdb_type_name;
use crate::comparison::constants::SEMI_STRUCTURED_TYPES;
use crate::comparison::models::{Mismatch, MismatchKind, RecordedResult, RowOrder};
use crate::values::models::ResultSet;

/// Compares names, declared types and values under the policy's comparator settings.
pub fn compare_with_replay(
    recorded: &RecordedResult,
    replayed: &ResultSet,
    order: RowOrder,
    comparator: &ComparatorPolicy,
) -> Result<(), Mismatch> {
    let fields = replayed.schema.fields();
    if fields.len() != recorded.columns.len() {
        return Err(Mismatch {
            kind: MismatchKind::ColumnCount,
            detail: format!(
                "recorded {} columns, replay returned {}",
                recorded.columns.len(),
                fields.len()
            ),
        });
    }
    for (index, (column, field)) in recorded.columns.iter().zip(fields.iter()).enumerate() {
        let names_match = match comparator.column_names {
            ColumnNameMatching::Exact => column.name == *field.name(),
            ColumnNameMatching::CaseInsensitive => column.name.eq_ignore_ascii_case(field.name()),
        };
        if !names_match {
            return Err(Mismatch {
                kind: MismatchKind::ColumnName,
                detail: format!(
                    "column {index}: recorded `{}`, replay returned `{}`",
                    column.name,
                    field.name()
                ),
            });
        }
        let duckdb_type = duckdb_type_name(field.data_type());
        if matching_type_equality(
            &comparator.type_equality,
            &column.snowflake_type,
            &duckdb_type,
        )
        .is_none()
        {
            return Err(Mismatch {
                kind: MismatchKind::ColumnType,
                detail: format!(
                    "column `{}`: no declared type equality admits Snowflake {} and DuckDB {duckdb_type}",
                    column.name, column.snowflake_type
                ),
            });
        }
    }
    compare_rows(
        &recorded.rows,
        replayed,
        &RowRules {
            order,
            float32_ulps: comparator.float_ulps.float32,
            float64_ulps: comparator.float_ulps.float64,
            ignore_zone_labels: comparator.timestamp_tz == TimestampTzComparison::Instant,
            json_columns: recorded
                .columns
                .iter()
                .map(|column| SEMI_STRUCTURED_TYPES.contains(&column.snowflake_type.as_str()))
                .collect(),
        },
    )
}
