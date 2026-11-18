/// One approved case's SQL, the root's screen.toml, and what `validate` must answer.
pub(crate) struct ValidateTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) allowlist: &'static str,
    pub(crate) expected_success: bool,
    pub(crate) expected_stderr_mention: &'static str,
}
