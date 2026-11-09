use crate::lineage::models::Nullability;

/// SQLite outputs: the query and each output's name, type and nullability.
pub(crate) struct SqliteOutputsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [(&'static str, Option<&'static str>, Nullability)],
}

/// A deeply nested SQLite query: its tree depth, outputs and findings, or the parser's refusal.
pub(crate) struct DeepSqliteTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_result: Result<(usize, usize, usize), &'static str>,
}

/// A nesting analysed at two depths, and how many times the shallow work the deep may take.
pub(crate) struct NestingWorkTestCase {
    pub(crate) description: &'static str,
    pub(crate) shape: fn(usize) -> String,
    pub(crate) expected_growth_at_most: usize,
}
