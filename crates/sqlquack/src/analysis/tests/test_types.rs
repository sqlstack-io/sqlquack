use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_rules::conversion::models::ConversionBudget;

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

pub(crate) struct PrettyLayoutTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_mismatches: Vec<String>,
}

pub(crate) struct DivergenceModeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) strict: bool,
    pub(crate) expected_outcome: DivergenceModeOutcome,
}

pub(crate) struct PythonModeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) strict: bool,
    pub(crate) runtime: bool,
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

pub(crate) struct LargeLiteralTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_converted: bool,
    pub(crate) expected_budget_seconds: u64,
}

pub(crate) struct CompositionTestCase {
    pub(crate) description: &'static str,
    pub(crate) columns: usize,
    pub(crate) depths: [usize; 3],
    pub(crate) expected_budget_seconds: u64,
}

pub(crate) struct BudgetTestCase {
    pub(crate) description: &'static str,
    pub(crate) budget: ConversionBudget,
    pub(crate) expected_category: Option<RefusalCategory>,
    pub(crate) expected_reason_fragment: &'static str,
}

pub(crate) struct LocationTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) declared: Option<&'static str>,
    pub(crate) budget: Option<ConversionBudget>,
    pub(crate) expected_places: Vec<(RefusalCategory, &'static str, [usize; 2])>,
}

pub(crate) struct FunctionLocationTestCase {
    pub(crate) description: &'static str,
    pub(crate) body: &'static str,
    pub(crate) expected_places: Vec<(RefusalCategory, &'static str, [usize; 2])>,
}

pub(crate) struct RowCountTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_rows: Result<usize, Vec<String>>,
}

pub(crate) struct RaisePlacesTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_places: Result<Vec<Vec<[usize; 2]>>, Vec<String>>,
}

pub(crate) struct RaisedPlacesTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_places: Result<Vec<[usize; 2]>, Vec<String>>,
}

pub(crate) struct ReformattedTestCase {
    pub(crate) description: &'static str,
    pub(crate) texts: &'static [&'static str],
    pub(crate) expected_identical: Result<bool, Vec<String>>,
}

pub(crate) struct StructuredTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_refusal: Result<usize, Vec<String>>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CollectedSummary {
    pub(crate) constructs: Vec<String>,
    pub(crate) possibly_incomplete: bool,
}

pub(crate) struct CollectedRefusalsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_summary: CollectedSummary,
}

pub(crate) struct CollectionIsolationTestCase {
    pub(crate) description: &'static str,
    pub(crate) converting: &'static [&'static str],
    pub(crate) refused: &'static str,
    /// Converting statements, refusals collected, and whether the SQL stayed byte-identical.
    pub(crate) expected_outcome: (usize, usize, bool),
}

pub(crate) struct FirstValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    /// The first value as text, or a fragment of the refusal reason or DuckDB error.
    pub(crate) expected_outcome: Result<&'static str, &'static str>,
}

/// A `FirstValueTestCase` whose SQL is built from shared parts.
pub(crate) struct BuiltFirstValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    /// The first value as text, or a fragment of the refusal reason or DuckDB error.
    pub(crate) expected_outcome: Result<&'static str, &'static str>,
}

pub(crate) struct ArgumentCopiesTestCase {
    pub(crate) description: &'static str,
    /// A query writing `ARGUMENT` once in its source.
    pub(crate) wrapper: &'static str,
    /// The most times the converted SQL may write the argument.
    pub(crate) expected_most_copies: usize,
}

pub(crate) struct EmittedGrowthTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_bindings: usize,
}

pub(crate) struct SessionTimeZoneTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) strict: bool,
    pub(crate) utc_session: bool,
    pub(crate) expected_outcome: DivergenceModeOutcome,
}

pub(crate) struct LoadOverrideTestCase {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) text: &'static str,
    pub(crate) expected_outcome: Result<(&'static str, usize), &'static str>,
}

pub(crate) struct OverrideCallsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) emitted: &'static str,
    pub(crate) expected_refused: bool,
}
