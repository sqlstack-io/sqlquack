use crate::expressions::tree::models::Expression;
use crate::traversal::models::ChildPathSegment;

pub(crate) struct OrderTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: Expression,
    pub(crate) expected_labels: &'static [&'static str],
}

pub(crate) struct SearchTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: Expression,
    pub(crate) kind: &'static str,
    pub(crate) expected_first: Option<&'static str>,
    pub(crate) expected_count: usize,
    pub(crate) expected_contains: bool,
}

pub(crate) struct DepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) expression: Expression,
    pub(crate) expected_depth: usize,
}

pub(crate) struct ChildPathTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_paths: Vec<Vec<ChildPathSegment>>,
}

pub(crate) struct ChildIdentityTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_child_count: usize,
}
