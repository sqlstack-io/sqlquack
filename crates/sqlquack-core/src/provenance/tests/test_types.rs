use std::collections::BTreeMap;

pub(crate) struct AttributionTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_rule_uses: BTreeMap<String, Vec<String>>,
}
