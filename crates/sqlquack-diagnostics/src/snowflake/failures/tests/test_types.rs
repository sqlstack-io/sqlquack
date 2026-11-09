use crate::failures::models::ErrorKind;

pub(crate) struct SnowflakeErrnoTestCase {
    pub(crate) description: &'static str,
    pub(crate) errno: i64,
    pub(crate) sqlstate: &'static str,
    pub(crate) expected_kind: Option<ErrorKind>,
}
