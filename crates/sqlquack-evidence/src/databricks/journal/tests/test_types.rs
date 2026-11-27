use sqlquack_compare::values::models::CanonicalValue;

pub(crate) struct CellTestCase {
    pub(crate) description: &'static str,
    pub(crate) type_text: &'static str,
    pub(crate) cell: Option<&'static str>,
    pub(crate) expected_value: Option<CanonicalValue>,
}

pub(crate) struct OutcomeTestCase {
    pub(crate) description: &'static str,
    pub(crate) fields: serde_json::Value,
    pub(crate) expected_outcome: Outcome,
}

/// What importing one journal line twice did to its pending case.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// The recorded error's SQLSTATE.
    pub(crate) sqlstate: Option<String>,
    /// Why the case stayed pending, or the expected part of it when that is present.
    pub(crate) pending_reason: Option<String>,
    /// How many results the second import found already recorded.
    pub(crate) already_recorded: usize,
}
