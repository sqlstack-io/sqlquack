use sqlquack_diagnostics::failures::models::ErrorKind;

/// A recorded Databricks error message and the kind verify reads it as.
pub(crate) struct FailureKindTestCase {
    pub(crate) description: &'static str,
    pub(crate) message: Option<&'static str>,
    pub(crate) expected_kind: Option<ErrorKind>,
}
