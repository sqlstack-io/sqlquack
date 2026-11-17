use crate::screen::models::{AllowedSameTree, MutationKind};

/// A text, and the (kind, token) of every variant `mutations` must write for it, in order.
pub(crate) struct MutationsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_mutations: Vec<(MutationKind, &'static str, &'static str)>,
}

/// A Databricks text, a one-token variant of it, and whether the two must parse to the same tree.
pub(crate) struct KnownCollapseTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) variant: &'static str,
    pub(crate) expected_same_tree: bool,
}

/// Allowlist entries, and what the screen of one same-tree case must report.
pub(crate) struct AllowlistTestCase {
    pub(crate) description: &'static str,
    pub(crate) entries: Vec<AllowedSameTree>,
    pub(crate) expected_flagged: usize,
    pub(crate) expected_allowed: usize,
    pub(crate) expected_stale: usize,
}
