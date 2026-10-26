use sqlquack_polyglot_check::corpora::models::CorpusEntry;

pub(crate) struct CorpusTestCase {
    pub(crate) description: &'static str,
    pub(crate) entries: Result<Vec<CorpusEntry>, String>,
    pub(crate) expected_outcome: ExactOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ExactOutcome {
    pub(crate) loaded: bool,
    pub(crate) differences: Vec<String>,
    pub(crate) tokens_lost: Vec<String>,
    pub(crate) type_names: Vec<String>,
    pub(crate) unlisted: Vec<String>,
    pub(crate) unexplained: Vec<String>,
}
