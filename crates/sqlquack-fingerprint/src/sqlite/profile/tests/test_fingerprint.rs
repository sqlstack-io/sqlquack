use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;
use crate::sqlite::profile::tests::test_types::StabilityTestCase;

#[test]
fn given_two_queries_when_fingerprinted_then_only_meaningful_edits_change_the_digest() {
    let test_cases = [
        StabilityTestCase {
            description: "SQLite comments and keyword case are ignored",
            dialect: FingerprintDialect::SQLite,
            first: "SELECT id FROM orders -- trailing",
            second: "select id\nfrom orders",
            expected_same: true,
        },
        StabilityTestCase {
            description: "SQLite identifier case is significant",
            dialect: FingerprintDialect::SQLite,
            first: "SELECT Id FROM orders",
            second: "SELECT id FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a column named Any keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select Any from orders",
            second: "select aNY from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named Any keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select * from Any",
            second: "select * from aNY",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named Any keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select order_id, Any + 2 from orders",
            second: "select order_id, aNY + 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a column named Some keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select Some from orders",
            second: "select sOME from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named Some keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select * from Some",
            second: "select * from sOME",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named Some keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select order_id, Some + 2 from orders",
            second: "select order_id, sOME + 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a column named For keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select For from orders",
            second: "select fOR from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named For keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select * from For",
            second: "select * from fOR",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named For keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select order_id, For + 2 from orders",
            second: "select order_id, fOR + 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a column named With keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select With from orders",
            second: "select wITH from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a table named With keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select * from With",
            second: "select * from wITH",
            expected_same: false,
        },
        StabilityTestCase {
            description: "an operand named With keeps its case",
            dialect: FingerprintDialect::SQLite,
            first: "select order_id, With + 2 from orders",
            second: "select order_id, wITH + 2 from orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "WITH before a common table expression still folds",
            dialect: FingerprintDialect::SQLite,
            first: "WITH recent AS (SELECT 1) SELECT * FROM recent",
            second: "with recent as (select 1) select * from recent",
            expected_same: true,
        },
        StabilityTestCase {
            description: "WITH before a column list and WITH RECURSIVE still fold",
            dialect: FingerprintDialect::SQLite,
            first: "WITH RECURSIVE n(x) AS (SELECT 1) SELECT x FROM n",
            second: "with RECURSIVE n(x) as (select 1) select x from n",
            expected_same: true,
        },
        StabilityTestCase {
            description: "WITH before a quoted common table expression name still folds",
            dialect: FingerprintDialect::SQLite,
            first: "WITH \"recent\" AS (SELECT 1) SELECT 1",
            second: "with \"recent\" as (select 1) select 1",
            expected_same: true,
        },
        StabilityTestCase {
            description: "FOR EACH ROW still folds",
            dialect: FingerprintDialect::SQLite,
            first: "CREATE TRIGGER t AFTER INSERT ON orders FOR EACH ROW BEGIN SELECT 1; END",
            second: "CREATE TRIGGER t AFTER INSERT ON orders for EACH ROW BEGIN SELECT 1; END",
            expected_same: true,
        },
    ];
    for test_case in test_cases {
        let first = query_fingerprint(test_case.first, test_case.dialect);
        let second = query_fingerprint(test_case.second, test_case.dialect);
        assert!(first.is_ok() && second.is_ok(), "{}", test_case.description);
        assert_eq!(
            first == second,
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}
