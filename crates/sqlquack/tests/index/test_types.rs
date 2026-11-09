#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IndexOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) text_to_boolean_files: Vec<String>,
    pub(crate) text_to_number_files: Vec<String>,
    pub(crate) hash_refusals: Vec<String>,
    pub(crate) converted: u64,
    pub(crate) refused: u64,
}

pub(crate) struct CorpusIndexTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_outcome: IndexOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CaseIndexOutcome {
    pub(crate) identical_runs: bool,
    pub(crate) boolean_text_cases_attributed: bool,
    pub(crate) number_to_boolean_not_attributed: bool,
}

pub(crate) struct CaseIndexTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_outcome: CaseIndexOutcome,
}
