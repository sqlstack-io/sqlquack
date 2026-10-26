use crate::refusals::models::RefusalCategory;
use crate::refusals::tests::helpers::{
    not_expressible_reasons, unmatched_refusals, unparsed_refusal,
};
use crate::refusals::tests::test_types::{ConstructRefusalTestCase, UnparsedTestCase};

#[test]
fn given_catalogue_when_refusing_constructs_then_every_occurrence_gets_its_own_refusal() {
    let test_cases = [
        ConstructRefusalTestCase {
            description: "an empty catalogue refuses everything as a missing rule",
            sql: "SELECT c.customer_id, COUNT(o.order_id) FROM customers c \
                  JOIN orders o ON o.customer_id = c.customer_id GROUP BY c.customer_id",
            inexpressible: &[],
            expected_unmatched_refusals: 0,
            expected_not_expressible_reasons: Vec::new(),
        },
        ConstructRefusalTestCase {
            description: "a declared construct is refused with the declared reason",
            sql: "SELECT RANDOM() FROM products",
            inexpressible: &["fn:RANDOM"],
            expected_unmatched_refusals: 0,
            expected_not_expressible_reasons: vec![
                "fn:RANDOM is declared inexpressible for this test".to_owned(),
            ],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            unmatched_refusals(test_case.sql, test_case.inexpressible),
            test_case.expected_unmatched_refusals,
            "{}",
            test_case.description
        );
        assert_eq!(
            not_expressible_reasons(test_case.sql, test_case.inexpressible),
            test_case.expected_not_expressible_reasons,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unparseable_sql_when_refusing_then_reports_parse_fidelity_with_location() {
    let test_cases = [UnparsedTestCase {
        description: "a SELECT with no projection",
        sql: "SELECT FROM WHERE",
        expected_category: RefusalCategory::ParseFidelity,
        expected_located: true,
    }];

    for test_case in test_cases {
        let refusal = unparsed_refusal(test_case.sql);
        assert_eq!(
            refusal.category, test_case.expected_category,
            "{}",
            test_case.description
        );
        assert_eq!(
            refusal.location.is_some(),
            test_case.expected_located,
            "{}",
            test_case.description
        );
    }
}
