/// Single-line SQL and its expected layout.
pub(crate) struct LayoutTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_layout: &'static str,
}

/// SQL holding text the layout does not model, and the reason its check gives.
pub(crate) struct FallbackTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_reason: &'static str,
}
