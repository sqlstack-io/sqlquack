use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_rules::conversion::models::ConversionBudget;

use crate::analysis::tests::helpers::{function_refusal_places, refusal_places};
use crate::analysis::tests::test_types::{FunctionLocationTestCase, LocationTestCase};

#[test]
fn given_refused_sql_when_analyzing_then_each_refusal_is_placed_at_its_construct() {
    let test_cases = [
        LocationTestCase {
            description: "a call no rule converts is placed at the call",
            sql: "SELECT\n  NOSUCHFN(amount) AS x\nFROM orders",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::MissingRule, "NOSUCHFN(amount)", [2, 3])],
        },
        LocationTestCase {
            description: "non-ASCII text before a construct counts in characters",
            sql: "SELECT 'héllo wörld' AS g,\n  'ünï' AS h, NOSUCHFN(amount) AS x\nFROM orders",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::MissingRule, "NOSUCHFN(amount)", [2, 15])],
        },
        LocationTestCase {
            description: "a call nested in other expressions is placed at itself",
            sql: "SELECT 1 + COALESCE(NOSUCHFN(amount), 0) AS x FROM orders",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::MissingRule, "NOSUCHFN(amount)", [1, 21])],
        },
        LocationTestCase {
            description: "a call in a CTE is placed inside the CTE",
            sql: "WITH c AS (\n  SELECT NOSUCHFN(amount) AS x FROM orders\n)\nSELECT x FROM c",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::MissingRule, "NOSUCHFN(amount)", [2, 10])],
        },
        LocationTestCase {
            description: "a call in a derived table is placed inside it",
            sql: "SELECT s.x\nFROM (SELECT NOSUCHFN(amount) AS x FROM orders) AS s",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::MissingRule, "NOSUCHFN(amount)", [2, 14])],
        },
        LocationTestCase {
            description: "a call in a WHERE subquery is placed inside it",
            sql: "SELECT amount FROM orders\nWHERE customer_id IN (SELECT NOSUCHFN(customer_id) FROM customers)",
            declared: None,
            budget: None,
            expected_places: vec![(
                RefusalCategory::MissingRule,
                "NOSUCHFN(customer_id)",
                [2, 30],
            )],
        },
        LocationTestCase {
            description: "an unknown column is placed at the column",
            sql: "SELECT amount,\n  o.missing_column\nFROM orders o",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::UnresolvedName, "o.missing_column", [2, 3])],
        },
        LocationTestCase {
            description: "an unknown relation is placed at the table",
            sql: "SELECT amount\nFROM nowhere",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::UnresolvedName, "nowhere", [2, 6])],
        },
        LocationTestCase {
            description: "an operator outside its type conditions is placed at the operator",
            sql: "SELECT note - 1 AS x FROM customers",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::TypeCondition, "note - 1", [1, 8])],
        },
        LocationTestCase {
            description: "a construct declared not expressible is placed at its use",
            sql: "SELECT\n  NOSUCHFN(amount) AS x\nFROM orders",
            declared: Some("fn:NOSUCHFN"),
            budget: None,
            expected_places: vec![(RefusalCategory::NotExpressible, "NOSUCHFN(amount)", [2, 3])],
        },
        LocationTestCase {
            description: "SQL that does not parse is placed at the failure",
            sql: "SELECT amount\nFROM orders WHERE (",
            declared: None,
            budget: None,
            expected_places: vec![(RefusalCategory::ParseFidelity, "(", [2, 19])],
        },
        LocationTestCase {
            description: "a join no rule converts is placed at the join",
            sql: "SELECT o.amount FROM orders o\nNATURAL JOIN customers c",
            declared: None,
            budget: None,
            expected_places: vec![(
                RefusalCategory::MissingRule,
                "NATURAL JOIN customers c",
                [2, 1],
            )],
        },
        LocationTestCase {
            description: "a set operation of unlike columns is placed at the whole operation",
            sql: "SELECT amount FROM orders\nUNION ALL SELECT note FROM customers",
            declared: None,
            budget: None,
            expected_places: vec![(
                RefusalCategory::TypeCondition,
                "SELECT amount FROM orders\nUNION ALL SELECT note FROM customers",
                [1, 1],
            )],
        },
        LocationTestCase {
            description: "a statement beyond the conversion budget is placed at the statement",
            sql: "SELECT amount\nFROM orders;",
            declared: None,
            budget: Some(ConversionBudget {
                milliseconds: u64::MAX,
                size_ratio: 0,
                minimum_bytes: 1,
            }),
            expected_places: vec![(
                RefusalCategory::ConversionBudget,
                "SELECT amount\nFROM orders",
                [1, 1],
            )],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            refusal_places(test_case.sql, (test_case.declared, test_case.budget)),
            test_case.expected_places,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_refused_function_body_when_analyzing_then_refusals_are_placed_in_the_body() {
    let test_cases = [
        FunctionLocationTestCase {
            description: "a call no rule converts is placed in the body, not the wrapped query",
            body: "amount +\n  NOSUCHFN(amount)",
            expected_places: vec![(RefusalCategory::MissingRule, "NOSUCHFN(amount)", [2, 3])],
        },
        FunctionLocationTestCase {
            description: "a body that does not parse is placed at its end",
            body: "amount + (",
            expected_places: vec![(RefusalCategory::ParseFidelity, "", [1, 11])],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            function_refusal_places(test_case.body),
            test_case.expected_places,
            "{}",
            test_case.description
        );
    }
}
