use crate::reparse::tests::helpers::{fidelity_outcome, fidelity_path, fidelity_report};
use crate::reparse::tests::test_types::FidelityTestCase;

#[test]
fn given_snowflake_statement_when_printing_back_then_classifies_the_tree_change() {
    let test_cases = [
        FidelityTestCase {
            description: "a plain query prints back to the same tree",
            sql: "SELECT o.order_id, o.amount::NUMBER(10, 2) AS amount FROM orders o \
                  WHERE o.status IN ('shipped', 'delivered') ORDER BY o.order_id",
            expected_outcome: "identical",
            expected_difference_path: None,
        },
        FidelityTestCase {
            description: "TOP prints back as LIMIT, a one-step canonicalisation",
            sql: "SELECT TOP 5 order_id FROM orders",
            expected_outcome: "canonicalised",
            expected_difference_path: Some(".select.limit".to_owned()),
        },
    ];

    for test_case in test_cases {
        let report = fidelity_report(test_case.sql);
        assert_eq!(
            fidelity_outcome(&report),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
        assert_eq!(
            fidelity_path(&report),
            test_case.expected_difference_path,
            "{}",
            test_case.description
        );
    }
}
