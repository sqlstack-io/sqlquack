//! Test case types of the databricks format tests.

/// One query formatted through `format_sql`, with its laid-out SQL or refusal reason.
pub(crate) struct FormatSqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: crate::format::models::FormatDialect,
    pub(crate) line_width: Option<usize>,
    /// The laid-out SQL, or the reason it is refused.
    pub(crate) expected_outcome: Result<&'static str, &'static str>,
}
