pub(crate) struct ChildIdentityTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: crate::expressions::tree::models::Expression,
    pub(crate) expected_child_count: usize,
}
