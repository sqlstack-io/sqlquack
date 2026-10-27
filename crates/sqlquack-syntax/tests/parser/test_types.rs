use serde_json::Value;
use sqlquack_syntax::Dialect;

pub(crate) struct FaithfulCallTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_call: (Value, Option<usize>, String),
}

pub(crate) struct ModeAgreementTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_same: bool,
}

pub(crate) struct ProjectionTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_value: Value,
}
