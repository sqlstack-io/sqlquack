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

pub(crate) struct StatementTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_value: Value,
}

pub(crate) struct LocatedTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_text: Result<&'static str, &'static str>,
}

pub(crate) struct LocatedAgreementTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_same: bool,
}

pub(crate) struct CastTargetTestCase {
    pub(crate) description: &'static str,
    pub(crate) type_text: &'static str,
    pub(crate) expected_same: bool,
}

pub(crate) struct AcceptanceTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_accepted: bool,
}

pub(crate) struct ScriptingStructureTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_fields: Vec<(&'static str, Value)>,
}

pub(crate) struct ScriptingVerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_error: Option<&'static str>,
}

pub(crate) struct ScriptingDialectTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_root: Result<&'static str, ()>,
}

pub(crate) struct ScriptingWalkTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_literals: Vec<&'static str>,
    pub(crate) expected_columns: Vec<&'static str>,
}
