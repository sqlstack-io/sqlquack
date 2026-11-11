use serde_json::Value;
use sqlquack_syntax::Dialect;

pub(crate) struct FaithfulCallTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_call: (Value, Option<usize>),
}

pub(crate) struct ProjectionKindTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_kind: &'static str,
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

pub(crate) struct ParseErrorTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_error_at: Option<&'static str>,
}

pub(crate) struct ParseMessageTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_message: &'static str,
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

pub(crate) struct SnowflakeVerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_kind: Option<&'static str>,
}

pub(crate) struct TriviaTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_trivia: Vec<(&'static str, &'static str)>,
}

pub(crate) struct TokenPresenceTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) wanted: &'static [(&'static str, &'static str)],
    pub(crate) expected_missing: Vec<(String, String)>,
}

pub(crate) struct LosslessTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_lossless: bool,
    pub(crate) expected_empty: bool,
}

pub(crate) struct TokenizeFailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) dialect: Dialect,
    pub(crate) expected_failed: bool,
}
