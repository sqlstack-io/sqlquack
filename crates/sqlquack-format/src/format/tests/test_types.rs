//! Test types the format test helpers share.

/// A corpus query whose SQLQuack outcome deliberately differs from SQLClean's recorded one.
pub(crate) struct Departure {
    pub(crate) sql: &'static str,
    /// SQLQuack's layout; `None` when SQLQuack refuses the query.
    pub(crate) formatted: Option<&'static str>,
    pub(crate) reason: &'static str,
    /// The recorded Snowflake evidence for SQLQuack's outcome: a `coverage/syntax` receipt or a `cases` case.
    pub(crate) receipt: Option<&'static str>,
}

/// The formatted, refused and departing counts of one corpus dialect, and every check that failed.
pub(crate) struct CorpusOutcome {
    pub(crate) formatted: usize,
    pub(crate) refused: usize,
    /// Departures whose query is in the corpus, each counted once however often it occurs.
    pub(crate) departed: usize,
    pub(crate) failures: Vec<String>,
}

/// One corpus query's outcome: whether it was formatted, and the checks it failed.
pub(crate) struct EntryOutcome {
    pub(crate) formatted: bool,
    pub(crate) failures: Vec<String>,
}
