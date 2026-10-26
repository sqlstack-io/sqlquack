use crate::checking::errors::CheckStage;

pub(crate) struct CheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_stage: Option<CheckStage>,
}
