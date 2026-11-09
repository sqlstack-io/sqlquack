use sqlquack_syntax::Dialect;

pub(crate) struct StrictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: Result<usize, bool>,
}

pub(crate) struct TolerantRangeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_ranges: Vec<(&'static str, bool)>,
}

pub(crate) struct TolerantTokenTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_tokens: (String, usize, bool),
}

pub(crate) struct TerminationTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) seconds: u64,
    pub(crate) expected_finished: bool,
}

pub(crate) struct LosslessTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_joined: &'static str,
}

pub(crate) struct StrictCoverageTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
}

pub(crate) struct CoverageTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) spans: &'static [(usize, usize)],
    pub(crate) expected: &'static str,
}
