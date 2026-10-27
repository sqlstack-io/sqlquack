use crate::functions::dispatch::models::ParserDispatchBehavior;

pub(crate) struct DispatchLookupTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_behavior: Option<ParserDispatchBehavior>,
}

pub(crate) struct DispatchSpecsTestCase {
    pub(crate) description: &'static str,
    pub(crate) names: Vec<&'static str>,
    pub(crate) expected_unique: usize,
}
