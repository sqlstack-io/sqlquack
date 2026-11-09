pub(crate) struct DuckdbTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_type: Option<&'static str>,
}
