/// Single-line SQL and its expected layout.
pub(crate) struct LayoutTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_layout: &'static str,
}
