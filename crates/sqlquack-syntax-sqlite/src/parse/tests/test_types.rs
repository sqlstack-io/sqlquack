//! Test case types for the SQLite parser.

/// An input nested far past the depth limit, built from a prefix, a core and a suffix.
pub(crate) struct NestingTestCase {
    pub(crate) description: &'static str,
    pub(crate) prefix: &'static str,
    pub(crate) core: &'static str,
    pub(crate) suffix: &'static str,
    pub(crate) expected_message: &'static str,
}
