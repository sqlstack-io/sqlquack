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
