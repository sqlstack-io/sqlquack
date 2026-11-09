pub(crate) struct SameIdTestCase {
    pub(crate) description: &'static str,
    pub(crate) left: String,
    pub(crate) right: String,
    pub(crate) expected_same: bool,
}

pub(crate) struct ReadBackTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_read: Option<usize>,
}

pub(crate) struct ManyRaisesTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_ids: usize,
    pub(crate) expected_budget_seconds: u64,
}
