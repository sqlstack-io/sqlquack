use std::collections::BTreeMap;

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_syntax::Dialect;

pub(crate) struct CorpusTestCase {
    pub(crate) description: &'static str,
    pub(crate) entries: Result<Vec<CorpusEntry>, String>,
    pub(crate) expected_outcome: ExactOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ExactOutcome {
    pub(crate) loaded: bool,
    pub(crate) rejected: BTreeMap<String, usize>,
    pub(crate) differences: Vec<String>,
    pub(crate) tokens_lost: Vec<String>,
    pub(crate) type_names: Vec<String>,
    pub(crate) unlisted: Vec<String>,
    pub(crate) unexplained: Vec<String>,
    pub(crate) located: Vec<String>,
}

pub(crate) struct RejectionTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) sql: &'static str,
    pub(crate) expected_rejected: usize,
    pub(crate) expected_differences: usize,
}

pub(crate) struct KeptQuotingTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_kept_quoting: usize,
    pub(crate) expected_differences: usize,
}

pub(crate) struct QuotingTreesTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) polyglot: String,
    pub(crate) sqlquack: String,
    pub(crate) expected_match: bool,
}
