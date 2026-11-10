use crate::lineage::tests::test_types::ExpectedTypedOutput;

pub(crate) struct OutputTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [ExpectedTypedOutput],
}

pub(crate) struct ProjectsStarTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_projects_star: bool,
}

pub(crate) struct ResolvedReadTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_sources: &'static [&'static str],
    pub(crate) expected_resolved: bool,
}

/// A query reading SELECT aliases, its outputs' sources and its findings as `code written`.
pub(crate) struct AliasReadTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_sources: &'static [(&'static str, &'static [&'static str])],
    pub(crate) expected_findings: &'static [&'static str],
}
