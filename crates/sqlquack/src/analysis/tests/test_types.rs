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
    pub(crate) converted: bool,
}

pub(crate) struct EquivalenceCasesTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_mismatches: Vec<String>,
}

pub(crate) struct DivergenceModeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) strict: bool,
    pub(crate) expected_outcome: DivergenceModeOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DivergenceModeOutcome {
    pub(crate) converted: bool,
    pub(crate) divergences: Vec<(String, Vec<String>)>,
    pub(crate) categories: Vec<RefusalCategory>,
}

#[derive(Debug)]
pub(crate) struct FunctionBodyTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) body: &'static str,
    pub(crate) call: &'static str,
    pub(crate) expected_values: Vec<String>,
}

pub(crate) struct EventResultTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) payload: &'static str,
    pub(crate) expected_values: Vec<String>,
}

pub(crate) struct CaseProvenanceTestCase {
    pub(crate) description: &'static str,
    pub(crate) reference: &'static str,
    pub(crate) expected_rules: Vec<String>,
}

pub(crate) struct GuardedConditionTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_gated: bool,
}
