pub(crate) struct PrettySqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_sql: &'static str,
}
