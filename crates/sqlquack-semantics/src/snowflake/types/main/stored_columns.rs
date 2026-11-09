//! `stored_columns`: a stored relation's columns as a later statement reads them.

use crate::schemas::models::Holding;
use crate::snowflake::typing::types::ColumnSpec;

/// The columns, each VARIANT another statement wrote from a typed build read back as JSON.
pub fn stored_columns(columns: Vec<ColumnSpec>) -> Vec<ColumnSpec> {
    columns
        .into_iter()
        .map(|column| ColumnSpec {
            holding: match column.holding {
                Holding::Json => Holding::Json,
                Holding::Variant | Holding::Chained => Holding::Chained,
            },
            ..column
        })
        .collect()
}
