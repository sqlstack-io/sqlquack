pub(crate) struct OverrideCallsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) emitted: &'static str,
    pub(crate) expected_refused: bool,
}

pub(crate) struct AnalyzeFunctionTestCase {
    pub(crate) description: &'static str,
    pub(crate) body: &'static str,
    pub(crate) expected_parameters: &'static [&'static str],
    pub(crate) expected_body_has: Option<&'static str>,
    pub(crate) expected_refusals: &'static [sqlquack_core::refusals::models::RefusalCategory],
}
