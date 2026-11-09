pub(crate) struct PostgresTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_type: Option<&'static str>,
}

/// A name called plainly, quoted and as an aggregate, and whether each call is window-only.
pub(crate) struct WindowOnlyTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_window_only: [bool; 3],
}
