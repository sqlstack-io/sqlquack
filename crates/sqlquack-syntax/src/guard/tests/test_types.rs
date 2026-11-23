use serde_json::error::Category;

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
