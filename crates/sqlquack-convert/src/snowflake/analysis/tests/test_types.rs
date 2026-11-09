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
    pub(crate) expected_refusals:
        &'static [sqlquack_diagnostics::refusals::models::RefusalCategory],
}

/// A function body, the session parameters it converts under, and what `convert_function` returns.
pub(crate) struct ConvertFunctionTestCase {
    pub(crate) description: &'static str,
    pub(crate) body: &'static str,
    pub(crate) parameters: &'static [(&'static str, &'static str)],
    /// The macro's parameters when it converts, or each diagnostic's code and the text its span covers.
    pub(crate) expected_outcome:
        Result<&'static [&'static str], &'static [(&'static str, Option<&'static str>)]>,
}
