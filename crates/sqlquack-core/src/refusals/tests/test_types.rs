use crate::classification::models::Construct;
use crate::refusals::models::{Disposition, RefusalCategory};
use crate::refusals::types::ConstructPolicy;

/// A catalogue declaring the listed constructs inexpressible and nothing else.
pub(crate) struct DeclaringPolicy {
    pub(crate) inexpressible: &'static [&'static str],
}

impl ConstructPolicy for DeclaringPolicy {
    fn disposition(&self, construct: &Construct) -> Disposition {
        let key = construct.key();
        self.inexpressible
            .iter()
            .find(|declared| **declared == key)
            .map_or(Disposition::Missing, |declared| {
                Disposition::NotExpressible {
                    reason: format!("{declared} is declared inexpressible for this test"),
                }
            })
    }
}

pub(crate) struct ConstructRefusalTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) inexpressible: &'static [&'static str],
    pub(crate) expected_unmatched_refusals: usize,
    pub(crate) expected_not_expressible_reasons: Vec<String>,
}

pub(crate) struct UnparsedTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_category: RefusalCategory,
    pub(crate) expected_located: bool,
}
