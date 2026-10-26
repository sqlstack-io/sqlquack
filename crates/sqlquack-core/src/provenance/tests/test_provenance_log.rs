use std::collections::BTreeMap;

use crate::provenance::tests::helpers::record_nested_rules;
use crate::provenance::tests::test_types::AttributionTestCase;

#[test]
fn given_nested_rule_scopes_when_recording_then_each_use_belongs_to_the_innermost_active_rule() {
    let test_cases = [AttributionTestCase {
        description: "uses go to the innermost rule; after it ends, to the outer rule; none outside",
        expected_rule_uses: BTreeMap::from([
            (
                "rule.inner".to_owned(),
                vec!["duckdb.inner_block".to_owned()],
            ),
            (
                "rule.outer".to_owned(),
                vec![
                    "duckdb.after_inner".to_owned(),
                    "snowflake.before_inner".to_owned(),
                ],
            ),
        ]),
    }];
    for test_case in test_cases {
        assert_eq!(
            record_nested_rules(),
            test_case.expected_rule_uses,
            "{}",
            test_case.description
        );
    }
}
