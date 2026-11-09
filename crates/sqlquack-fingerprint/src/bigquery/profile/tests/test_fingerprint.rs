use crate::bigquery::profile::tests::test_types::{DialectNameTestCase, StabilityTestCase};
use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;

#[test]
fn given_two_queries_when_fingerprinted_then_only_meaningful_edits_change_the_digest() {
    let test_cases = [
        StabilityTestCase {
            description: "BigQuery keyword case is ignored, JOIN, CASE, WHEN, END, BY and LIMIT included",
            dialect: FingerprintDialect::BigQuery,
            first: "SELECT o.id, CASE WHEN o.x > 1 THEN 'a' END AS k FROM ds.orders AS o JOIN ds.c AS c ON o.id = c.id ORDER BY o.id LIMIT 5",
            second: "select o.id, case when o.x > 1 then 'a' end as k from ds.orders as o join ds.c as c on o.id = c.id order by o.id limit 5",
            expected_same: true,
        },
        StabilityTestCase {
            description: "BigQuery table name case is significant",
            dialect: FingerprintDialect::BigQuery,
            first: "SELECT id FROM Orders",
            second: "SELECT id FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a BigQuery backquoted keyword is a name, so its case is significant",
            dialect: FingerprintDialect::BigQuery,
            first: "SELECT `Select` FROM t",
            second: "SELECT `select` FROM t",
            expected_same: false,
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

#[test]
fn given_dialect_names_when_resolved_then_each_spelling_names_its_dialect() {
    let test_cases = [DialectNameTestCase {
        description: "BigQuery",
        name: "bigquery",
        expected_dialect: Ok(FingerprintDialect::BigQuery),
    }];
    for test_case in test_cases {
        assert_eq!(
            FingerprintDialect::from_name(test_case.name),
            test_case.expected_dialect,
            "{}",
            test_case.description
        );
    }
}
