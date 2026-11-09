use crate::failures::models::DuckdbFailure;

pub(crate) struct DuckdbMessageTestCase {
    pub(crate) description: &'static str,
    pub(crate) message: String,
    pub(crate) expected_failure: Option<DuckdbFailure>,
}
