use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::main::canonical_tokens::canonical_tokens;
use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;
use crate::fingerprint::tests::test_types::{
    DialectNameTestCase, FailureTestCase, GoldenTestCase, NestingTestCase, StabilityTestCase,
    TokensTestCase,
};

/// Nesting depth of the pathological inputs.
const NESTING: usize = 32_000;
/// Room for linear work at that depth in a debug build, but not for a quadratic path.
const NESTING_LIMIT: std::time::Duration = std::time::Duration::from_secs(2);

#[test]
fn given_recorded_queries_when_fingerprinted_then_digests_never_change() {
    let test_cases = [
        GoldenTestCase {
            description: "generic token stream",
            dialect: FingerprintDialect::Generic,
            sql: "SELECT id, name FROM orders",
            expected_digest: "264bcb1f81ce5ba25394a052016edc265aefba85c0698f616d3037f9d872d2bd",
        },
        GoldenTestCase {
            description: "DuckDB dollar quote and cast",
            dialect: FingerprintDialect::DuckDB,
            sql: "SELECT o.id, $$Customer's note$$ AS note, amount::DECIMAL(10, 2) AS amount FROM orders AS o",
            expected_digest: "07786643d9593498ac2b2394e6801cb578b23ba0f9a0e70dbcdaf0d3a3cbf9db",
        },
        GoldenTestCase {
            description: "PostgreSQL tagged dollar quote",
            dialect: FingerprintDialect::PostgreSQL,
            sql: "SELECT o.id, $tag$it's$tag$ AS note, amount::NUMERIC AS amount FROM orders AS o",
            expected_digest: "8db865c6f239b990c917754df176bd3c9b2c0c322b9af1437298ec98b67241f9",
        },
        GoldenTestCase {
            description: "Snowflake path access and QUALIFY",
            dialect: FingerprintDialect::Snowflake,
            sql: "SELECT o.id, payload:customer.name::STRING AS customer FROM orders AS o QUALIFY ROW_NUMBER() OVER (PARTITION BY o.id ORDER BY o.id) = 1",
            expected_digest: "c75f7b5931a18e1a074ffa7ab113c256fc838285af75a3f9feb62d61cc76d94d",
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
            description: "comments are ignored",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT id -- order key\nFROM orders /* all */",
            second: "SELECT id FROM orders",
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
            description: "Snowflake path key case is significant",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT payload:customerId FROM orders",
            second: "SELECT payload:customerid FROM orders",
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
            description: "quoted call name case is significant",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT \"MyFunc\"(id) FROM orders",
            second: "SELECT \"myfunc\"(id) FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "PostgreSQL multi-character operator is one token",
            dialect: FingerprintDialect::PostgreSQL,
            first: "SELECT a |>> b FROM orders",
            second: "SELECT a |> > b FROM orders",
            expected_same: false,
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
            description: "Snowflake double-slash comments are ignored",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT id // order key\nFROM orders",
            second: "SELECT id FROM orders",
            expected_same: true,
        },
        StabilityTestCase {
            description: "generic keyword case is ignored and identifier case is not",
            dialect: FingerprintDialect::Generic,
            first: "select Id from orders",
            second: "SELECT id FROM orders",
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
            description: "quoted identifier case is significant",
            dialect: FingerprintDialect::Snowflake,
            first: "SELECT \"Id\" FROM orders",
            second: "SELECT \"ID\" FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "number spelling is significant",
            dialect: FingerprintDialect::DuckDB,
            first: "SELECT 1.0 AS amount",
            second: "SELECT 1.00 AS amount",
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
        TokensTestCase {
            description: "PostgreSQL's scanner keeps a multi-character operator and a dollar body whole",
            dialect: FingerprintDialect::PostgreSQL,
            sql: "select a |>> $t$Body$t$ -- note",
            expected_tokens: &["SELECT", "a", "|>>", "$t$Body$t$"],
        },
        TokensTestCase {
            description: "a word reserved only in Snowflake folds only there",
            dialect: FingerprintDialect::Generic,
            sql: "select qualify from t",
            expected_tokens: &["SELECT", "qualify", "FROM", "t"],
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
    let test_cases = [
        FailureTestCase {
            description: "an unterminated string under the lossless tokenizer",
            dialect: FingerprintDialect::DuckDB,
            sql: "SELECT 'open FROM orders",
            expected_fragment: "Unterminated string",
        },
        FailureTestCase {
            description: "an unterminated string under the PostgreSQL scanner",
            dialect: FingerprintDialect::PostgreSQL,
            sql: "SELECT 'open FROM orders",
            expected_fragment: "SQL tokenization failed: unterminated",
        },
    ];
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
fn given_pathological_nesting_when_fingerprinted_then_time_stays_linear() {
    let test_cases = [
        NestingTestCase {
            description: "deeply nested parentheses",
            dialect: FingerprintDialect::DuckDB,
            sql: format!("SELECT 1 + {}1{}", "(".repeat(NESTING), ")".repeat(NESTING)),
            expected_within_limit: true,
        },
        NestingTestCase {
            description: "deeply nested calls",
            dialect: FingerprintDialect::PostgreSQL,
            sql: format!("SELECT {}1{}", "f(".repeat(NESTING), ")".repeat(NESTING)),
            expected_within_limit: true,
        },
        NestingTestCase {
            description: "unclosed calls",
            dialect: FingerprintDialect::Snowflake,
            sql: format!("SELECT {}", "f(".repeat(NESTING)),
            expected_within_limit: true,
        },
    ];
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
fn given_dialect_names_when_resolved_then_both_postgres_spellings_agree() {
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
        DialectNameTestCase {
            description: "no named dialect",
            name: "generic",
            expected_dialect: Ok(FingerprintDialect::Generic),
        },
        DialectNameTestCase {
            description: "a dialect SQLQuack does not tokenize",
            name: "tsql",
            expected_dialect: Err(FingerprintError::UnknownDialect("tsql".into())),
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
