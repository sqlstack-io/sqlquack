//! Type aliases of a run.

/// One result row, each value as DuckDB renders it as text.
pub type Row = Vec<Option<String>>;
/// The result's columns, each its name and DuckDB type.
pub type Columns = Vec<(String, String)>;
