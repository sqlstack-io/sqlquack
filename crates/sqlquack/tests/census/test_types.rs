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
