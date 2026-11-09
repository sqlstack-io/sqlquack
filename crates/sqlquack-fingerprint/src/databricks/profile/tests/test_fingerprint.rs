use crate::databricks::profile::tests::test_types::{SameAsTestCase, StabilityTestCase};
use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;

#[test]
fn given_two_queries_when_fingerprinted_then_only_meaningful_edits_change_the_digest() {
    let test_cases = [
        StabilityTestCase {
            description: "a column named Any keeps its case",
            first: "select Any from orders",
            second: "select aNY from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named Any keeps its case",
            first: "select * from Any",
            second: "select * from aNY",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named Any keeps its case",
            first: "select order_id, Any ^ 2 from orders",
            second: "select order_id, aNY ^ 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a column named Some keeps its case",
            first: "select Some from orders",
            second: "select sOME from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named Some keeps its case",
            first: "select * from Some",
            second: "select * from sOME",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named Some keeps its case",
            first: "select order_id, Some ^ 2 from orders",
            second: "select order_id, sOME ^ 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a column named Order keeps its case",
            first: "select Order from orders",
            second: "select oRDER from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named Order keeps its case",
            first: "select * from Order",
            second: "select * from oRDER",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named Order keeps its case",
            first: "select order_id, Order ^ 2 from orders",
            second: "select order_id, oRDER ^ 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a qualified column named Order keeps its case",
            first: "select o.Order from orders o",
            second: "select o.oRDER from orders o",
            expected_same: false,
        },
        StabilityTestCase {
            description: "ORDER BY still folds",
            first: "SELECT order_id FROM orders ORDER by order_id",
            second: "select order_id from orders order by order_id",
            expected_same: true,
        },
        StabilityTestCase {
            description: "a column named Order before ORDER BY keeps its case while the clause folds",
            first: "SELECT Order FROM orders ORDER by Order",
            second: "select Order from orders order by Order",
            expected_same: true,
        },
        StabilityTestCase {
            description: "ANY and SOME before a subquery still fold",
            first: "SELECT 1 FROM orders WHERE id = ANY (SELECT id FROM t) OR id = SOME(SELECT id FROM t)",
            second: "select 1 from orders where id = any (select id from t) or id = some(select id from t)",
            expected_same: true,
        },
    ];
    for test_case in test_cases {
        let first = query_fingerprint(test_case.first, FingerprintDialect::Databricks);
        let second = query_fingerprint(test_case.second, FingerprintDialect::Databricks);
        assert!(first.is_ok() && second.is_ok(), "{}", test_case.description);
        assert_eq!(
            first == second,
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_without_keyword_named_identifiers_when_fingerprinted_then_digest_is_the_generic_one() {
    let test_cases = [
        SameAsTestCase {
            description: "ORDER BY and GROUP BY in mixed case",
            sql: "Select Id, count(*) From orders Group By Id Order By Id",
            expected_same_as: FingerprintDialect::Generic,
        },
        SameAsTestCase {
            description: "ANY and SOME before a subquery",
            sql: "select 1 from orders where id = Any (select id from t) and id <> sOme(select 1)",
            expected_same_as: FingerprintDialect::Generic,
        },
        SameAsTestCase {
            description: "an Order alias and a qualified Order column",
            sql: "select o.Order as Order from orders o",
            expected_same_as: FingerprintDialect::Generic,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            query_fingerprint(test_case.sql, FingerprintDialect::Databricks),
            query_fingerprint(test_case.sql, test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}
