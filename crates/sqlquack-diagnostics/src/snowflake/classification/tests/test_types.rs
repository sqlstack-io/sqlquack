pub(crate) struct ConstructTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_keys: &'static [&'static str],
}

pub(crate) struct CompletenessTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_unclassified_nodes: usize,
    pub(crate) expected_unlocated_constructs: usize,
}

pub(crate) struct LocationTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) key: &'static str,
    pub(crate) expected_line_and_column: Option<[usize; 2]>,
}
