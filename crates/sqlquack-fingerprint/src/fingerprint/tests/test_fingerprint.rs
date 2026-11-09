use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::main::canonical_tokens::canonical_tokens;
use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;
use crate::fingerprint::tests::test_types::{
    DialectNameTestCase, GoldenTestCase, StabilityTestCase, TokensTestCase,
};

#[test]
fn given_recorded_queries_when_fingerprinted_then_digests_never_change() {
    let test_cases = [GoldenTestCase {
        description: "generic token stream",
        dialect: FingerprintDialect::Generic,
        sql: "SELECT id, name FROM orders",
        expected_digest: "264bcb1f81ce5ba25394a052016edc265aefba85c0698f616d3037f9d872d2bd",
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
            description: "generic keyword case is ignored and identifier case is not",
            dialect: FingerprintDialect::Generic,
            first: "select Id from orders",
            second: "SELECT id FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "different queries produce different hashes",
            dialect: FingerprintDialect::Generic,
            first: "SELECT id FROM orders",
            second: "SELECT id FROM customers",
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
fn given_sql_when_canonicalized_then_only_keywords_and_called_names_fold() {
    let test_cases = [TokensTestCase {
        description: "a word reserved only in Snowflake folds only there",
        dialect: FingerprintDialect::Generic,
        sql: "select qualify from t",
        expected_tokens: &["SELECT", "qualify", "FROM", "t"],
    }];
    for test_case in test_cases {
        let tokens = canonical_tokens(test_case.sql, test_case.dialect);
        assert_eq!(
            tokens
                .as_deref()
                .map(|tokens| tokens.iter().map(String::as_str).collect::<Vec<_>>()),
            Ok(test_case.expected_tokens.to_vec()),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_dialect_names_when_resolved_then_each_spelling_names_its_dialect() {
    let test_cases = [
        DialectNameTestCase {
            description: "no named dialect",
            name: "generic",
            expected_dialect: Ok(FingerprintDialect::Generic),
        },
        DialectNameTestCase {
            description: "a dialect SQLQuack does not tokenize",
            name: "oracle",
            expected_dialect: Err(FingerprintError::UnknownDialect("oracle".into())),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            FingerprintDialect::from_name(test_case.name),
            test_case.expected_dialect,
            "{}",
            test_case.description
        );
    }
}
