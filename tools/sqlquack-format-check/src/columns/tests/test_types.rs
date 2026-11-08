//! Test case types of columns.

/// Queries laid out for DuckDB whose output columns DuckDB must bind the same before and after.
pub(super) struct CompareColumnsTestCase {
    pub(super) description: &'static str,
    /// The format corpus dialect whose queries are included; empty for none.
    pub(super) corpus_dialect: &'static str,
    pub(super) queries: &'static [&'static str],
    /// Layouts, at no line width and at the corpus width, whose source DuckDB binds.
    pub(super) expected_bound: usize,
    pub(super) expected_unbound: usize,
    pub(super) expected_refused: usize,
}
