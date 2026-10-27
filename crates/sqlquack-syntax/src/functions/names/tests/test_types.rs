use crate::tokens::models::DialectType;

pub(crate) struct NameListTestCase {
    pub(crate) description: &'static str,
    pub(crate) names: &'static [&'static str],
    pub(crate) checked: fn(&[&'static str]) -> Vec<&'static str>,
    pub(crate) expected_failures: Vec<&'static str>,
}

pub(crate) struct NameLookupTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) dialect: Option<DialectType>,
    pub(crate) lookup: fn(&str, Option<DialectType>) -> bool,
    pub(crate) expected_found: bool,
}
