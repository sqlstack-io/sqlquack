pub(crate) struct PolicyTestCase {
    pub(crate) description: &'static str,
    pub(crate) replace: &'static str,
    pub(crate) with: &'static str,
    pub(crate) expected_error: Option<&'static str>,
}

pub(crate) struct TypeEqualityTestCase {
    pub(crate) description: &'static str,
    pub(crate) snowflake: &'static str,
    pub(crate) duckdb: &'static str,
    pub(crate) expected_reason: Option<&'static str>,
}
