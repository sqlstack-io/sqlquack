use crate::failures::models::{DuckdbFailure, ErrorKind};

pub(crate) struct DuckdbMessageTestCase {
    pub(crate) description: &'static str,
    pub(crate) message: String,
    pub(crate) expected_failure: Option<DuckdbFailure>,
}

pub(crate) struct SnowflakeErrnoTestCase {
    pub(crate) description: &'static str,
    pub(crate) errno: i64,
    pub(crate) sqlstate: &'static str,
    pub(crate) expected_kind: Option<ErrorKind>,
}
