use serde_json::Value;

use crate::cli::_helpers::local::postgres_verdict;
use crate::cli::tests::test_types::PostgresVerdictTestCase;

#[test]
fn given_sql_when_the_postgres_parser_judges_it_then_gaps_are_never_parsed() {
    let test_cases = [
        PostgresVerdictTestCase {
            description: "a query the parser builds a full tree for",
            sql: "SELECT a FROM t WHERE a > 1",
            expected_verdict: "accepted",
            expected_opaque: Some(false),
            expected_tree: true,
        },
        PostgresVerdictTestCase {
            description: "a syntax error",
            sql: "SELECT 1 FROM",
            expected_verdict: "rejected",
            expected_opaque: None,
            expected_tree: false,
        },
        PostgresVerdictTestCase {
            description: "a syntax error raised by a grammar action",
            sql: "COPY t TO 'f' WHERE a > 1",
            expected_verdict: "rejected",
            expected_opaque: None,
            expected_tree: false,
        },
        PostgresVerdictTestCase {
            description: "a statement holding a construct the parser keeps as an opaque gap",
            sql: "SELECT TREAT(1 AS int)",
            expected_verdict: "accepted",
            expected_opaque: Some(true),
            expected_tree: true,
        },
        PostgresVerdictTestCase {
            description: "a parser error PostgreSQL does not report as 42601 is still refused",
            sql: "SELECT E'\\u12'",
            expected_verdict: "rejected",
            expected_opaque: None,
            expected_tree: false,
        },
    ];
    for test_case in test_cases {
        let verdict: Value =
            serde_json::to_value(postgres_verdict(test_case.sql)).expect(test_case.description);
        assert_eq!(
            (
                verdict["verdict"].as_str(),
                verdict["opaque"].as_bool(),
                verdict.get("tree_sha256").is_some(),
            ),
            (
                Some(test_case.expected_verdict),
                test_case.expected_opaque,
                test_case.expected_tree,
            ),
            "{}",
            test_case.description
        );
    }
}
