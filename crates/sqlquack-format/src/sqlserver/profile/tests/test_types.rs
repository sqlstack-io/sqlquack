//! Test case types of the sqlserver format tests.

use crate::format::tests::test_types::Departure;

/// A generated, deeply nested query, and the refusal it gets, if any.
pub(crate) struct DeepSqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: fn() -> String,
    /// The start of the refusal reason; `None` when the query is laid out.
    pub(crate) expected_refusal: Option<&'static str>,
}

/// One dialect of the format corpus with its expected formatted and refused counts.
pub(crate) struct FormatCorpusTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: &'static str,
    pub(crate) expected_formatted: usize,
    pub(crate) expected_refused: usize,
    /// Corpus queries whose SQLQuack outcome deliberately differs from SQLClean's.
    pub(crate) departures: &'static [Departure],
}

/// One query formatted through `format_sql`, with its laid-out SQL or refusal reason.
pub(crate) struct FormatSqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: crate::format::models::FormatDialect,
    pub(crate) line_width: Option<usize>,
    /// The laid-out SQL, or the reason it is refused.
    pub(crate) expected_outcome: Result<&'static str, &'static str>,
}

/// A corpus statement SQLQuack refuses, with SQL Server's recorded verdicts and the formatter's refusal.
pub(crate) struct SqlServerReceiptTestCase {
    /// The statement, as recorded.
    pub(crate) description: &'static str,
    /// What each recorded SQL Server run reported: its first error number and message, or `None` when it was accepted.
    pub(crate) expected_runs: &'static [Option<(u64, &'static str)>],
    /// The start of the formatter's refusal reason.
    pub(crate) expected_refusal: &'static str,
}

/// A wide query whose SQL Server formatting time is bounded by Snowflake's on the same text, so a quadratic pass fails it.
pub(crate) struct TimedSqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: fn() -> String,
    /// How many times Snowflake's time SQL Server may take.
    pub(crate) expected_max_ratio: u32,
}
