pub(crate) struct DatabricksOutputsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [(&'static str, Option<&'static str>)],
}
