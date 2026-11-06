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
