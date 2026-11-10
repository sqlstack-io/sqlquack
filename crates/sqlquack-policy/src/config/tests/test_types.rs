pub(crate) struct ConfigTestCase {
    pub(crate) description: &'static str,
    pub(crate) text: &'static str,
    pub(crate) expected_error: Option<&'static str>,
}
