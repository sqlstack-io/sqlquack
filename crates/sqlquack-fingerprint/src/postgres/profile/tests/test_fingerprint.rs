use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::main::canonical_tokens::canonical_tokens;
use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;
use crate::postgres::profile::tests::test_types::{
    DialectNameTestCase, FailureTestCase, GoldenTestCase, NestingTestCase, StabilityTestCase,
    TokensTestCase,
};

/// Nesting depth of the pathological inputs.
const NESTING: usize = 32_000;
/// Room for linear work at that depth in a debug build, but not for a quadratic path.
const NESTING_LIMIT: std::time::Duration = std::time::Duration::from_secs(2);

#[test]
fn given_recorded_queries_when_fingerprinted_then_digests_never_change() {
    let test_cases = [GoldenTestCase {
        description: "PostgreSQL tagged dollar quote",
        dialect: FingerprintDialect::PostgreSQL,
        sql: "SELECT o.id, $tag$it's$tag$ AS note, amount::NUMERIC AS amount FROM orders AS o",
        expected_digest: "8db865c6f239b990c917754df176bd3c9b2c0c322b9af1437298ec98b67241f9",
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
            description: "PostgreSQL multi-character operator is one token",
            dialect: FingerprintDialect::PostgreSQL,
            first: "SELECT a |>> b FROM orders",
            second: "SELECT a |> > b FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "PostgreSQL comments, layout and keyword case are ignored",
            dialect: FingerprintDialect::PostgreSQL,
            first: "SELECT o.id -- key\nFROM orders AS o WHERE o.id > 0",
            second: "select o.id from orders as o /* all */ where o.id > 0",
            expected_same: true,
        },
        StabilityTestCase {
            description: "PostgreSQL dollar-quoted body is significant",
            dialect: FingerprintDialect::PostgreSQL,
            first: "SELECT $tag$it's$tag$ AS note",
            second: "SELECT $tag$it s$tag$ AS note",
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
        description: "PostgreSQL's scanner keeps a multi-character operator and a dollar body whole",
        dialect: FingerprintDialect::PostgreSQL,
        sql: "select a |>> $t$Body$t$ -- note",
        expected_tokens: &["SELECT", "a", "|>>", "$t$Body$t$"],
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
fn given_untokenizable_sql_when_fingerprinted_then_the_tokenizer_message_is_reported() {
    let test_cases = [FailureTestCase {
        description: "an unterminated string under the PostgreSQL scanner",
        dialect: FingerprintDialect::PostgreSQL,
        sql: "SELECT 'open FROM orders",
        expected_fragment: "SQL tokenization failed: unterminated",
    }];
    for test_case in test_cases {
        let message: String = query_fingerprint(test_case.sql, test_case.dialect)
            .err()
            .map(|error: FingerprintError| error.to_string())
            .unwrap_or_default();
        assert!(
            message.contains(test_case.expected_fragment),
            "{}: {message}",
            test_case.description
        );
    }
}

#[test]
#[ignore = "timing"]
fn given_pathological_nesting_when_fingerprinted_then_time_stays_linear() {
    let test_cases = [NestingTestCase {
        description: "deeply nested calls",
        dialect: FingerprintDialect::PostgreSQL,
        sql: format!("SELECT {}1{}", "f(".repeat(NESTING), ")".repeat(NESTING)),
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

#[test]
fn given_dialect_names_when_resolved_then_each_spelling_names_its_dialect() {
    let test_cases = [
        DialectNameTestCase {
            description: "the short PostgreSQL name",
            name: "postgres",
            expected_dialect: Ok(FingerprintDialect::PostgreSQL),
        },
        DialectNameTestCase {
            description: "the long PostgreSQL name",
            name: "postgresql",
            expected_dialect: Ok(FingerprintDialect::PostgreSQL),
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
