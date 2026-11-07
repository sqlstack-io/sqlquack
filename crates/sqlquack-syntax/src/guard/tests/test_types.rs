use serde_json::error::Category;

use crate::guard::models::ComplexityGuardOptions;

pub(crate) struct GuardJsonTestCase {
    pub(crate) description: &'static str,
    pub(crate) json: &'static str,
    pub(crate) expected_max_parser_depth: Result<Option<usize>, Category>,
}

pub(crate) struct GuardRoundTripTestCase {
    pub(crate) description: &'static str,
    pub(crate) max_parser_depth: Option<usize>,
    pub(crate) expected_max_parser_depth: Option<usize>,
}

pub(crate) struct GuardStatsTestCase {
    pub(crate) description: &'static str,
    pub(crate) options: ComplexityGuardOptions,
    pub(crate) expected_error: Result<(), String>,
}

pub(crate) struct GuardLimitJsonTestCase {
    pub(crate) description: &'static str,
    pub(crate) field: &'static str,
    pub(crate) expected_problems: Vec<String>,
}

pub(crate) struct GuardKeyTestCase {
    pub(crate) description: &'static str,
    pub(crate) json: &'static str,
    pub(crate) expected_accepted: bool,
}

pub(crate) struct AstDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: fn() -> String,
    /// None when the default guard accepts the query; otherwise a fragment of the error.
    pub(crate) expected_error: Option<&'static str>,
}

pub(crate) struct TypeDepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: crate::Dialect,
    /// The query with its type nested `n` levels deep.
    pub(crate) sql: fn(n: usize) -> String,
    /// The guard code at each of [`TYPE_DEPTHS`] levels, or None where the parse succeeds.
    pub(crate) expected_errors: [Option<&'static str>; 4],
}

/// The nesting depths each [`TypeDepthTestCase`] is parsed at.
pub(crate) const TYPE_DEPTHS: [usize; 4] = [512, 513, 20_000, 100_000];

pub(crate) struct TreeOperationTestCase {
    pub(crate) description: &'static str,
    /// The operation on the parsed tree; true when it ran to the end.
    pub(crate) operation: fn(Vec<crate::expressions::tree::models::Expression>) -> bool,
    pub(crate) expected_completed: bool,
}
