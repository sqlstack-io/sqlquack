pub(crate) struct DuckdbTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_type: Option<&'static str>,
}

pub(crate) struct ResolvedReadTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_sources: &'static [&'static str],
    pub(crate) expected_resolved: bool,
}

pub(crate) struct SingleAnalysisTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [&'static str],
    pub(crate) expected_finding_count: usize,
}
