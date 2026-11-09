use crate::verify::models::Verdict;

/// A recorded case: its input column `k` holding `cells`, and the key argument's source.
pub(crate) struct RecordedGet {
    pub(crate) cells: &'static [&'static str],
    pub(crate) key_family: &'static str,
    pub(crate) key_source: &'static str,
    pub(crate) verdict: Verdict,
}

/// What coverage reports for the one declared rule.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RuleGaps {
    pub(crate) shape_gaps: Vec<String>,
    pub(crate) undeclared_shapes: Vec<String>,
    pub(crate) hazard_gaps: Vec<String>,
}

pub(crate) struct CoverageTestCase {
    pub(crate) description: &'static str,
    pub(crate) key_families: &'static [&'static str],
    pub(crate) recorded: RecordedGet,
    pub(crate) expected_gaps: RuleGaps,
}

/// Groups of the input column `g` and the values fed to an aggregate.
pub(crate) struct GroupTestCase {
    pub(crate) description: &'static str,
    pub(crate) groups: &'static [&'static str],
    pub(crate) values: &'static [&'static str],
    pub(crate) expected_hazard_gaps: Vec<String>,
}

pub(crate) struct ProbeTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_outside: Vec<String>,
}

/// Coverage of a rule whose text key has an unrecordable hazard beside two recorded ones.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UnrecordableSplit {
    pub(crate) hazard_gaps: Vec<String>,
    pub(crate) unrecordable_hazards: Vec<String>,
    pub(crate) hazards: usize,
}

pub(crate) struct UnrecordableTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_split: UnrecordableSplit,
}
