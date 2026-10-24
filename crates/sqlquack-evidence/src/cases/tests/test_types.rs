pub(crate) struct HashTestCase {
    pub(crate) description: &'static str,
    pub(crate) replace: &'static str,
    pub(crate) with: &'static str,
    pub(crate) expected_same_hash: bool,
}

pub(crate) struct LoadTestCase {
    pub(crate) description: &'static str,
    pub(crate) replace: &'static str,
    pub(crate) with: &'static str,
    pub(crate) expected_problem: Option<&'static str>,
}

pub(crate) struct SourceSqlTestCase {
    pub(crate) description: &'static str,
    pub(crate) reference: &'static str,
    pub(crate) expected_sql: &'static str,
}
