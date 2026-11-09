//! Test case types of the bigquery format tests.

use crate::format::tests::test_types::Departure;

/// A statement BigQuery was recorded rejecting, with the syntax error it reported.
pub(crate) struct BigQueryReceiptTestCase {
    /// The statement, as recorded.
    pub(crate) description: &'static str,
    /// The recorded message and `line:column` of BigQuery's syntax error.
    pub(crate) expected_rejection: (&'static str, &'static str),
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

/// A query whose formatting time must grow linearly with its size, so a quadratic pass fails it.
pub(crate) struct ScalingSqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: crate::format::models::FormatDialect,
    /// The query at a given size.
    pub(crate) sql: fn(usize) -> String,
    /// The small and the large size timed.
    pub(crate) sizes: (usize, usize),
    /// How many times the small size's time the large one may take.
    pub(crate) expected_max_ratio: u32,
}
