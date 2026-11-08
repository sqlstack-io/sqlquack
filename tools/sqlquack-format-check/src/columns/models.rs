//! Data models of columns.

/// Layouts counted by whether DuckDB binds their source, and each one whose columns changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnsOutcome {
    /// Layouts whose source DuckDB binds, so their columns were compared.
    pub bound: usize,
    /// Layouts whose source DuckDB does not bind, such as ones naming missing tables.
    pub unbound: usize,
    /// Layouts the formatter refused.
    pub refused: usize,
    /// Each layout whose bound columns, or whether it binds, differ from its source.
    pub changes: Vec<String>,
}
