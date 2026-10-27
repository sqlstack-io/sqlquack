use sqlquack_syntax::Dialect;

pub(crate) struct TriviaTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_trivia: Vec<(&'static str, &'static str)>,
}

pub(crate) struct TokenPresenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) wanted: &'static [(&'static str, &'static str)],
    pub(crate) expected_missing: Vec<(String, String)>,
}

pub(crate) struct LosslessTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_lossless: bool,
    pub(crate) expected_empty: bool,
}

pub(crate) struct TokenizeFailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_failed: bool,
}
