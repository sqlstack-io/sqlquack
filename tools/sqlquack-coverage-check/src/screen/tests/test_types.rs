use crate::screen::models::{AllowedSameTree, LexicalSpelling, MutationKind};

/// A text, and the (kind, token) of every variant `mutations` must write for it, in order.
pub(crate) struct MutationsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_mutations: Vec<(MutationKind, &'static str, &'static str)>,
}

/// A text, a variant of it, the lexer ranges used, and whether the variant is written.
pub(crate) struct GluedTokenTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) variant: &'static str,
    pub(crate) lexed: fn(&str) -> Vec<(usize, usize)>,
    pub(crate) expected_written: bool,
}

/// A text with a comment, one variant of it, and whether that variant's change lies inside the comment's body.
pub(crate) struct CommentBodyTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) variant: &'static str,
    pub(crate) expected_untokenized: bool,
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
    pub(crate) lexical: Vec<LexicalSpelling>,
    /// Whether the run screens every form, so entries for unscreened forms are judged.
    pub(crate) full: bool,
    pub(crate) expected_flagged: usize,
    pub(crate) expected_allowed: usize,
    pub(crate) expected_stale: usize,
    pub(crate) expected_unpinned: usize,
}
