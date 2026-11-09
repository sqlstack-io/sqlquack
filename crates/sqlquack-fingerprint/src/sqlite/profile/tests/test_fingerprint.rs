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
