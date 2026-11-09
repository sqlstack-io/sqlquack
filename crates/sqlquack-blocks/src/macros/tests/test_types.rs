pub(crate) struct HelperListTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_heads: Vec<&'static str>,
}

pub(crate) struct CreationTestCase {
    pub(crate) description: &'static str,
    pub(crate) runs: usize,
    pub(crate) expected_failures: Vec<String>,
}

pub(crate) struct ExplainTestCase {
    pub(crate) description: &'static str,
    pub(crate) key: &'static str,
    pub(crate) expected_first_line: Option<&'static str>,
}

pub(crate) struct UnsitedRaiseTestCase {
    pub(crate) description: &'static str,
    pub(crate) marker: &'static str,
    pub(crate) expected_unsited: Vec<String>,
}

pub(crate) struct PinnedDefinitionTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_md5: Option<&'static str>,
}
