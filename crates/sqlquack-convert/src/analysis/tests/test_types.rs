pub(crate) struct OverrideCallsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) emitted: &'static str,
    pub(crate) expected_refused: bool,
}
