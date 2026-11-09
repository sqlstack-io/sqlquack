use crate::duckdb::profile::tests::test_types::{
    FailureTestCase, GoldenTestCase, NestingTestCase, StabilityTestCase, TokensTestCase,
};
use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::main::canonical_tokens::canonical_tokens;
use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;

/// Nesting depth of the pathological inputs.
const NESTING: usize = 32_000;
/// Room for linear work at that depth in a debug build, but not for a quadratic path.
const NESTING_LIMIT: std::time::Duration = std::time::Duration::from_secs(2);

#[test]
fn given_recorded_queries_when_fingerprinted_then_digests_never_change() {
    let test_cases = [
        GoldenTestCase {
            description: "DuckDB dollar quote and cast",
            dialect: FingerprintDialect::DuckDB,
            sql: "SELECT o.id, $$Customer's note$$ AS note, amount::DECIMAL(10, 2) AS amount FROM orders AS o",
            expected_digest: "07786643d9593498ac2b2394e6801cb578b23ba0f9a0e70dbcdaf0d3a3cbf9db",
        },
        GoldenTestCase {
            description: "DuckDB lower-case keywords and SQLBuild placeholder",
            dialect: FingerprintDialect::DuckDB,
            sql: "select o.id from __ref(\"orders\") as o where o.id > 0",
            expected_digest: "fda88b9b0f26486bdd25b8e3bab5a982fbb387901ab229091b17772d044aebd0",
        },
    ];
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
            description: "whitespace and line breaks are ignored",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT  id  FROM  orders",
            second: "select\n  id\nfrom orders",
            expected_same: true,
        },
        StabilityTestCase {
            description: "keyword and built-in function case is ignored",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT COUNT(id) FROM orders WHERE id > 0 LIMIT 1",
            second: "select count(id) from orders where id > 0 limit 1",
            expected_same: true,
        },
        StabilityTestCase {
            description: "unquoted identifier case is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT Id FROM Orders",
            second: "SELECT id FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "alias case is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT 1 AS Foo",
            second: "SELECT 1 AS foo",
            expected_same: false,
        },
        StabilityTestCase {
            description: "unquoted call name case is ignored",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT MyFunc(id) FROM orders",
            second: "SELECT myfunc(id) FROM orders",
            expected_same: true,
        },
        StabilityTestCase {
            description: "CTE column-list name case is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "WITH Tree(id) AS (SELECT 1) SELECT id FROM orders",
            second: "WITH tree(id) AS (SELECT 1) SELECT id FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "SQLBuild placeholder case is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT id FROM __ref(\"orders\")",
            second: "SELECT id FROM __REF(\"orders\")",
            expected_same: false,
        },
        StabilityTestCase {
            description: "string literal whitespace is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT 'a  b' AS code",
            second: "SELECT 'a b' AS code",
            expected_same: false,
        },
        StabilityTestCase {
            description: "number spelling is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT 1.0 AS amount",
            second: "SELECT 1.00 AS amount",
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
    let test_cases = [
        TokensTestCase {
            description: "keywords and a called function fold; identifiers and aliases keep their case",
            dialect: FingerprintDialect::DuckDB,
            sql: "select count(Id) as Total from Orders",
            expected_tokens: &[
                "SELECT", "COUNT", "(", "Id", ")", "AS", "Total", "FROM", "Orders",
            ],
        },
        TokensTestCase {
            description: "a reserved word after a dot or AS is a name",
            dialect: FingerprintDialect::DuckDB,
            sql: "select o.select as from_ from o",
            expected_tokens: &["SELECT", "o", ".", "select", "AS", "from_", "FROM", "o"],
        },
        TokensTestCase {
            description: "placeholder calls and INTO column lists keep their case; INSERT is unreserved in DuckDB",
            dialect: FingerprintDialect::DuckDB,
            sql: "insert into Orders(id) select id from __ref(\"x\")",
            expected_tokens: &[
                "insert", "INTO", "Orders", "(", "id", ")", "SELECT", "id", "FROM", "__ref", "(",
                "\"x\"", ")",
            ],
        },
        TokensTestCase {
            description: "a CTE column list is not a call, but an unclosed call still is",
            dialect: FingerprintDialect::DuckDB,
            sql: "with Tree(a) as (select f(1)) select g(",
            expected_tokens: &[
                "WITH", "Tree", "(", "a", ")", "AS", "(", "SELECT", "F", "(", "1", ")", ")",
                "SELECT", "G", "(",
            ],
        },
    ];
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
        description: "an unterminated string under the lossless tokenizer",
        dialect: FingerprintDialect::DuckDB,
        sql: "SELECT 'open FROM orders",
        expected_fragment: "Unterminated string",
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
        description: "deeply nested parentheses",
        dialect: FingerprintDialect::DuckDB,
        sql: format!("SELECT 1 + {}1{}", "(".repeat(NESTING), ")".repeat(NESTING)),
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
