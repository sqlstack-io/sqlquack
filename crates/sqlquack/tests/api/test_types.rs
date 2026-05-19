pub(crate) struct AnalyzeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_diagnostic_count: usize,
}
