pub(crate) struct LookupTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_title: Option<&'static str>,
}

pub(crate) struct EntryCheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) findings: fn() -> Vec<String>,
    pub(crate) expected_findings: &'static [&'static str],
}
