pub(crate) struct CommandTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) mention: &'static str,
    pub(crate) expected_outcome: CommandOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CommandOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) wrote_pending_files: bool,
    pub(crate) stderr_mentions: bool,
}

pub(crate) struct DivergenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) case_marker: &'static str,
    pub(crate) policy_default: &'static str,
    pub(crate) expected_outcome: DivergenceOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DivergenceOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) verdict: String,
}

pub(crate) struct ReviewTestCase {
    pub(crate) description: &'static str,
    pub(crate) file: &'static str,
    pub(crate) expected_lines: &'static [&'static str],
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ReviewOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) missing_lines: Vec<String>,
}

pub(crate) struct ErrorCheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) recorded_error: Option<(i64, &'static str)>,
    pub(crate) audited_message: Option<&'static str>,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_outcome: ErrorCheckOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErrorCheckOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) verdict: String,
    pub(crate) error_check: String,
    pub(crate) snowflake_message: Option<String>,
}

#[derive(Debug)]
pub(crate) struct FunctionMacroTestCase {
    pub(crate) description: &'static str,
    pub(crate) returns: &'static str,
    pub(crate) body: &'static str,
    pub(crate) call: &'static str,
    pub(crate) expected_outcome: FunctionMacroOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FunctionMacroOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) values: Vec<String>,
}

pub(crate) struct CoverageTestCase {
    pub(crate) description: &'static str,
    pub(crate) key_families: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_outcome: CoverageOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CoverageOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) shape_gaps: Vec<String>,
    pub(crate) hazard_gaps: Vec<String>,
}

pub(crate) struct PendingFlagsTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_flags: Vec<(String, Option<bool>)>,
}
