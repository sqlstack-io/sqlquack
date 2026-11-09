pub(crate) struct ResolveNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: (Option<&'static str>, &'static str),
    pub(crate) expected_outcome: Result<&'static str, &'static str>,
}
