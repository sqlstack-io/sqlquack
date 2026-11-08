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
    pub(super) receipt: Option<&'static str>,
}

/// The formatted, refused and departing counts of one corpus dialect, and every check that failed.
pub(super) struct CorpusOutcome {
    pub(super) formatted: usize,
    pub(super) refused: usize,
    /// Departures whose query is in the corpus, each counted once however often it occurs.
    pub(super) departed: usize,
    pub(super) failures: Vec<String>,
}

/// One corpus query's outcome: whether it was formatted, and the checks it failed.
pub(super) struct EntryOutcome {
    pub(super) formatted: bool,
    pub(super) failures: Vec<String>,
}

/// A generated, deeply nested query, and the refusal it gets in each gated dialect, if any.
pub(super) struct DeepSqlTestCase {
    pub(super) description: &'static str,
    pub(super) sql: fn() -> String,
    /// The dialects it is formatted in.
    pub(super) dialects: &'static [crate::format::models::FormatDialect],
    /// The start of the refusal reason; `None` when the query is laid out.
    pub(super) expected_refusal: Option<&'static str>,
}

/// One query formatted through `format_sql`, with its laid-out SQL or refusal reason.
pub(super) struct FormatSqlTestCase {
    pub(super) description: &'static str,
    pub(super) sql: &'static str,
    pub(super) dialect: crate::format::models::FormatDialect,
    pub(super) line_width: Option<usize>,
    /// The laid-out SQL, or the reason it is refused.
    pub(super) expected_outcome: Result<&'static str, &'static str>,
}

/// A corpus statement SQLQuack refuses, with SQL Server's recorded verdicts and the formatter's refusal.
pub(super) struct SqlServerReceiptTestCase {
    /// The statement, as recorded.
    pub(super) description: &'static str,
    /// What each recorded SQL Server run reported: its first error number and message, or `None` when it was accepted.
    pub(super) expected_runs: &'static [Option<(u64, &'static str)>],
    /// The start of the formatter's refusal reason.
    pub(super) expected_refusal: &'static str,
}

/// A wide query whose SQL Server formatting time is bounded by Snowflake's on the same text, so a quadratic pass fails it.
pub(super) struct TimedSqlTestCase {
    pub(super) description: &'static str,
    pub(super) sql: fn() -> String,
    /// How many times Snowflake's time SQL Server may take.
    pub(super) expected_max_ratio: u32,
}

/// A query whose formatting time must grow linearly with its size, so a quadratic pass fails it.
pub(super) struct ScalingSqlTestCase {
    pub(super) description: &'static str,
    pub(super) dialect: crate::format::models::FormatDialect,
    /// The query at a given size.
    pub(super) sql: fn(usize) -> String,
    /// The small and the large size timed.
    pub(super) sizes: (usize, usize),
    /// How many times the small size's time the large one may take.
    pub(super) expected_max_ratio: u32,
}

/// A statement BigQuery was recorded rejecting, with the syntax error it reported.
pub(super) struct BigQueryReceiptTestCase {
    /// The statement, as recorded.
    pub(super) description: &'static str,
    /// The recorded message and `line:column` of BigQuery's syntax error.
    pub(super) expected_rejection: (&'static str, &'static str),
}
