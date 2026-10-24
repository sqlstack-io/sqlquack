use crate::refusals::models::RefusalCategory;
use crate::refusals::tests::helpers::{
    difference, infidelity_category, not_expressible_reasons, unmatched_refusals, unparsed_refusal,
};
use crate::refusals::tests::test_types::{
    ConstructRefusalTestCase, InfidelityTestCase, UnparsedTestCase,
};
use crate::reparse::models::FidelityOutcome;

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
            inexpressible: &["node:random"],
            expected_unmatched_refusals: 0,
            expected_not_expressible_reasons: vec![
                "node:random is declared inexpressible for this test".to_owned(),
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

#[test]
fn given_fidelity_outcome_when_refusing_then_only_identical_trees_pass() {
    let test_cases = [
        InfidelityTestCase {
            description: "an identical print-back",
            outcome: FidelityOutcome::Identical,
            expected_category: None,
        },
        InfidelityTestCase {
            description: "an unproven canonicalisation",
            outcome: FidelityOutcome::Canonicalised {
                difference: difference(),
            },
            expected_category: Some(RefusalCategory::ParseFidelity),
        },
        InfidelityTestCase {
            description: "a tree that never settles",
            outcome: FidelityOutcome::Unstable {
                difference: difference(),
            },
            expected_category: Some(RefusalCategory::ParseFidelity),
        },
        InfidelityTestCase {
            description: "a tree that does not print",
            outcome: FidelityOutcome::PrintFailed {
                message: "printer failed".to_owned(),
            },
            expected_category: Some(RefusalCategory::ParseFidelity),
        },
        InfidelityTestCase {
            description: "a print that does not parse",
            outcome: FidelityOutcome::ReparseFailed {
                message: "parser failed".to_owned(),
            },
            expected_category: Some(RefusalCategory::ParseFidelity),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            infidelity_category(&test_case.outcome),
            test_case.expected_category,
            "{}",
            test_case.description
        );
    }
}
