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

pub(crate) struct RowOrdinalTestCase {
    pub(crate) description: &'static str,
    pub(crate) row: usize,
    pub(crate) expected_ordinal: Option<usize>,
}

pub(crate) struct RowSourceTestCase {
    pub(crate) description: &'static str,
    pub(crate) reference: &'static str,
    pub(crate) replace: &'static str,
    pub(crate) with: &'static str,
    pub(crate) expected_rows: Option<usize>,
}
