use crate::lineage::models::Nullability;
use crate::lineage::tests::test_types::ExpectedTypedOutput;

pub(crate) struct ClickhouseJoinTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_output: (Option<&'static str>, Nullability),
    pub(crate) expected_codes: &'static [&'static str],
}

pub(crate) struct ClickhouseTypeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_output: ExpectedTypedOutput,
}
