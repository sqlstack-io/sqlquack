use sqlquack_core::refusals::models::RefusalCategory;

use crate::analysis::tests::helpers::summarise;
use crate::analysis::tests::test_types::{AnalysisSummary, AnalyzeTestCase};

#[test]
fn given_sql_and_builtin_catalogue_when_analyzing_then_everything_is_refused_with_a_reason() {
    let test_cases = [
        AnalyzeTestCase {
            description: "every construct of a faithful query is a missing rule",
            sql: "SELECT c.customer_id, SUM(o.amount) AS total FROM customers c \
                  JOIN orders o ON o.customer_id = c.customer_id GROUP BY c.customer_id",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::MissingRule],
                unrefused_constructs: 0,
                unlocated_construct_refusals: 0,
            },
        },
        AnalyzeTestCase {
            description: "a canonicalising print-back adds a parse-fidelity refusal",
            sql: "SELECT TOP 5 order_id FROM orders",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::MissingRule, RefusalCategory::ParseFidelity],
                unrefused_constructs: 0,
                unlocated_construct_refusals: 0,
            },
        },
        AnalyzeTestCase {
            description: "unparseable SQL is one parse-fidelity refusal",
            sql: "SELECT FROM WHERE",
            expected_summary: AnalysisSummary {
                parsed: false,
                statements: 0,
                categories: vec![RefusalCategory::ParseFidelity],
                unrefused_constructs: 0,
                unlocated_construct_refusals: 0,
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            summarise(test_case.sql),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}
