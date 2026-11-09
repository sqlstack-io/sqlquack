use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;
use crate::snowflake::profile::tests::test_types::{
    GoldenTestCase, NestingTestCase, StabilityTestCase,
};

/// Nesting depth of the pathological inputs.
const NESTING: usize = 32_000;
/// Room for linear work at that depth in a debug build, but not for a quadratic path.
const NESTING_LIMIT: std::time::Duration = std::time::Duration::from_secs(2);

#[test]
fn given_recorded_queries_when_fingerprinted_then_digests_never_change() {
    let test_cases = [GoldenTestCase {
        description: "Snowflake path access and QUALIFY",
        dialect: FingerprintDialect::Snowflake,
        sql: "SELECT o.id, payload:customer.name::STRING AS customer FROM orders AS o QUALIFY ROW_NUMBER() OVER (PARTITION BY o.id ORDER BY o.id) = 1",
        expected_digest: "c75f7b5931a18e1a074ffa7ab113c256fc838285af75a3f9feb62d61cc76d94d",
    }];
    for test_case in test_cases {
        assert_eq!(
            query_fingerprint(test_case.sql, test_case.dialect).as_deref(),
            Ok(test_case.expected_digest),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_two_queries_when_fingerprinted_then_only_meaningful_edits_change_the_digest() {
    let test_cases = [
        StabilityTestCase {
            description: "comments are ignored",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT id -- order key\nFROM orders /* all */",
            second: "SELECT id FROM orders",
            expected_same: true,
        },
        StabilityTestCase {
            description: "Snowflake path key case is significant",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT payload:customerId FROM orders",
            second: "SELECT payload:customerid FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "quoted call name case is significant",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT \"MyFunc\"(id) FROM orders",
            second: "SELECT \"myfunc\"(id) FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "Snowflake double-slash comments are ignored",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT id // order key\nFROM orders",
            second: "SELECT id FROM orders",
            expected_same: true,
        },
        StabilityTestCase {
            description: "quoted identifier case is significant",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT \"Id\" FROM orders",
            second: "SELECT \"ID\" FROM orders",
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
#[ignore = "timing"]
fn given_pathological_nesting_when_fingerprinted_then_time_stays_linear() {
    let test_cases = [NestingTestCase {
        description: "unclosed calls",
        dialect: FingerprintDialect::Snowflake,
        sql: format!("SELECT {}", "f(".repeat(NESTING)),
        expected_within_limit: true,
    }];
    for test_case in test_cases {
        let started = std::time::Instant::now();
        let _ = query_fingerprint(&test_case.sql, test_case.dialect);
        assert_eq!(
            started.elapsed() < NESTING_LIMIT,
            test_case.expected_within_limit,
            "{}",
            test_case.description
        );
    }
}
