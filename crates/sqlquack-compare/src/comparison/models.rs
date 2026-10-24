//! Comparison inputs and outcomes.

use std::fmt::{self, Display};

use crate::values::models::ResultSet;

/// A recorded result column: its name and its declared Snowflake type, such as `NUMBER(38,0)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedColumn {
    /// The result column name.
    pub name: String,
    /// The Snowflake type as recorded.
    pub snowflake_type: String,
}

/// A recorded warehouse result.
#[derive(Clone, Debug)]
pub struct RecordedResult {
    /// The declared columns, in result order.
    pub columns: Vec<RecordedColumn>,
    /// The values.
    pub rows: ResultSet,
}

/// Whether row order is part of the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOrder {
    /// Rows must appear in the same order.
    Ordered,
    /// Rows compare as a multiset.
    Bag,
}

/// What differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MismatchKind {
    /// The number of columns.
    ColumnCount,
    /// A column name.
    ColumnName,
    /// A column type no declared equality admits.
    ColumnType,
    /// The number of rows.
    RowCount,
    /// A value.
    Value,
    /// A value the comparator cannot decode, which never matches.
    Unsupported,
}

/// The first difference found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// What differs.
    pub kind: MismatchKind,
    /// Where and how.
    pub detail: String,
}

impl Display for Mismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}
