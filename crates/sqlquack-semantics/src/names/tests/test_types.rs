pub(crate) struct RelationColumnsTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_outcome: Result<Vec<&'static str>, &'static str>,
}
