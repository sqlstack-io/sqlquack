use crate::check::models::LocalVerdict;

/// A worker script given two SQL texts, and the verdict it must yield on each.
pub(crate) struct WorkerTestCase {
    pub(crate) description: &'static str,
    /// A `sh -c` script standing in for the parse worker.
    pub(crate) script: &'static str,
    pub(crate) bound_ms: u64,
    pub(crate) expected_verdicts: [LocalVerdict; 2],
}

/// A dialect whose parser path is not committed, and what `parser_tree` must give for it.
pub(crate) struct ParserTreeTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: &'static str,
    pub(crate) parser_path: &'static str,
    /// `error`, `none` or `tree`.
    pub(crate) expected_outcome: &'static str,
}

/// A SQL text and the PostgreSQL verdict the checker must give it.
pub(crate) struct PostgresVerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    /// `accepted` or `rejected`.
    pub(crate) expected_verdict: &'static str,
    /// Whether an accepted parse is opaque; `None` for a rejection.
    pub(crate) expected_opaque: Option<bool>,
    /// Whether the verdict carries a tree fingerprint.
    pub(crate) expected_tree: bool,
}
