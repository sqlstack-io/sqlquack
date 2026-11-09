//! `schema_columns`: a schema relation's columns as a statement reads them, marks applied.

use crate::snowflake::types::models::{ColumnSpec, TypedLeaves};
use crate::snowflake::typing::models::SnowflakeType;

/// The columns, proven free unless marked, entering in `typed` the leaves marked ones may hold.
pub(in crate::snowflake) fn schema_columns(
    columns: &[ColumnSpec],
    typed: &TypedLeaves,
) -> Vec<ColumnSpec> {
    if columns.iter().any(|column| column.typed_leaves) {
        typed.enter();
    }
    if columns.iter().any(|column| column.decimal_leaves) {
        typed.enter_decimal();
    }
    columns
        .iter()
        .map(|column| ColumnSpec {
            untainted: !column.typed_leaves && !column.decimal_leaves,
            sourced: !column.typed_leaves
                && !column.decimal_leaves
                && column.data_type == SnowflakeType::SemiStructured,
            ..column.clone()
        })
        .collect()
}
