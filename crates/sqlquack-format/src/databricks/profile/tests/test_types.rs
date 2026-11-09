//! Test case types of the databricks format tests.

use crate::format::tests::test_types::Departure;

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
