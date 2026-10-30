pub(crate) struct BinaryTestCase {
    pub(crate) description: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) expected_outcome: BinaryOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BinaryOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) files: Option<u64>,
    pub(crate) unresolved_name: Option<u64>,
    pub(crate) completion_reported: bool,
    pub(crate) slowest_reported: bool,
    pub(crate) names_printed: bool,
}

pub(crate) struct SourceVariantTestCase {
    pub(crate) description: &'static str,
    pub(crate) payload: &'static str,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: SourceVariantOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SourceVariantOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) convertible: Option<u64>,
    pub(crate) labelled: Option<u64>,
}

pub(crate) struct UpstreamTestCase {
    pub(crate) description: &'static str,
    pub(crate) typed: bool,
    pub(crate) flags: &'static [&'static str],
    pub(crate) expected_outcome: UpstreamOutcome,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UpstreamOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) unit_testable: Option<u64>,
    pub(crate) strict: Option<u64>,
    pub(crate) labels: Vec<String>,
    pub(crate) text_counts: Option<String>,
    /// The plan's unit-testable and strict upper bounds.
    pub(crate) plan_bounds: Option<[u64; 2]>,
}
