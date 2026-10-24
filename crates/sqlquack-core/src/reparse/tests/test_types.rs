pub(crate) struct FidelityTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: &'static str,
    pub(crate) expected_difference_path: Option<String>,
}

pub(crate) struct PrinterTestCase {
    pub(crate) description: &'static str,
    pub(crate) duckdb_sql: &'static str,
    pub(crate) expected_failure: Option<&'static str>,
    pub(crate) expected_difference_path: Option<String>,
}
