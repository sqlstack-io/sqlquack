pub(crate) struct DatabricksOutputsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outputs: &'static [(&'static str, Option<&'static str>)],
}

pub(crate) struct AllQuantifierTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_same_as: &'static str,
}
