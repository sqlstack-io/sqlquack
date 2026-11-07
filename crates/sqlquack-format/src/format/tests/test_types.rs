//! Test case types of format.

/// One dialect of the format corpus with its expected formatted and refused counts.
pub(super) struct FormatCorpusTestCase {
    pub(super) description: &'static str,
    pub(super) dialect: &'static str,
    pub(super) expected_formatted: usize,
    pub(super) expected_refused: usize,
    /// Corpus queries whose SQLQuack outcome deliberately differs from SQLClean's.
    pub(super) departures: &'static [Departure],
}

/// A corpus query whose SQLQuack outcome deliberately differs from SQLClean's recorded one.
pub(super) struct Departure {
    pub(super) sql: &'static str,
    /// SQLQuack's layout; `None` when SQLQuack refuses the query.
    pub(super) formatted: Option<&'static str>,
    pub(super) reason: &'static str,
    /// The recorded Snowflake evidence for SQLQuack's outcome: a `coverage/syntax` receipt or a `cases` case.
    pub(super) receipt: &'static str,
}

/// The formatted, refused and departing counts of one corpus dialect, and every check that failed.
pub(super) struct CorpusOutcome {
    pub(super) formatted: usize,
    pub(super) refused: usize,
    /// Corpus queries that matched a departure.
    pub(super) departed: usize,
    pub(super) failures: Vec<String>,
}

/// One corpus query's outcome: whether it was formatted or departs, and the checks it failed.
pub(super) struct EntryOutcome {
    pub(super) formatted: bool,
    pub(super) departed: bool,
    pub(super) failures: Vec<String>,
}

/// A generated, deeply nested Snowflake query, and the refusal it gets, if any.
pub(super) struct DeepSqlTestCase {
    pub(super) description: &'static str,
    pub(super) sql: fn() -> String,
    /// The start of the refusal reason; `None` when the query is laid out.
    pub(super) expected_refusal: Option<&'static str>,
}

/// One query formatted through `format_sql`, with its laid-out SQL or refusal reason.
pub(super) struct FormatSqlTestCase {
    pub(super) description: &'static str,
    pub(super) sql: &'static str,
    pub(super) dialect: sqlquack_syntax::Dialect,
    pub(super) line_width: Option<usize>,
    /// The laid-out SQL, or the reason it is refused.
    pub(super) expected_outcome: Result<&'static str, &'static str>,
}
