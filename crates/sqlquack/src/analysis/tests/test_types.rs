use sqlquack_core::refusals::models::RefusalCategory;

pub(crate) struct AnalyzeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_summary: AnalysisSummary,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AnalysisSummary {
    pub(crate) parsed: bool,
    pub(crate) statements: usize,
    pub(crate) categories: Vec<RefusalCategory>,
    pub(crate) unrefused_constructs: usize,
    pub(crate) unlocated_construct_refusals: usize,
}
