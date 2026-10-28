use crate::checking::errors::CheckStage;

pub(crate) struct CheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_stage: Option<CheckStage>,
}

/// One check over the given relations, run after the previous case on the same engine.
pub(crate) struct RelationsTestCase {
    pub(crate) description: &'static str,
    pub(crate) relations: &'static [&'static str],
    pub(crate) expected_stage: Option<CheckStage>,
}
