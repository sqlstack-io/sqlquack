/// One file of a plan input: its own roots as category and construct, and whether incomplete.
pub(crate) type PlannedFile = (&'static [(&'static str, &'static str)], bool);

/// One step: its constructs, and its unit-testable, strict and possibly incomplete counts.
pub(crate) type StepSummary = (Vec<String>, [usize; 3]);

pub(crate) struct StepsTestCase {
    pub(crate) description: &'static str,
    pub(crate) files: &'static [PlannedFile],
    pub(crate) expected_steps: Vec<StepSummary>,
}
