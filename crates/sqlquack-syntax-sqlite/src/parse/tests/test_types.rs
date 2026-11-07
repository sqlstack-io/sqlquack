//! Test case types for the SQLite parser.

/// An input nested far past the depth limit, built from a prefix, a core and a suffix.
pub(crate) struct NestingTestCase {
    pub(crate) description: &'static str,
    pub(crate) prefix: &'static str,
    pub(crate) core: &'static str,
    pub(crate) suffix: &'static str,
    pub(crate) expected_message: &'static str,
}

/// An input with the verdict SQLite 3.53.4 gives it: its error, or the gaps of an accepted parse.
pub(crate) struct VerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_verdict: Result<&'static [&'static str], &'static str>,
}
