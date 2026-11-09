use crate::duckdb::profile::tests::test_types;
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::corpus_outcome;
use crate::format::tests::test_types::Departure;

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [test_types::FormatCorpusTestCase {
        description: "DuckDB identity corpus",
        dialect: "duckdb",
        expected_formatted: 189,
        expected_refused: 0,
        departures: &[
            Departure {
                sql: "SELECT str[0:1]",
                formatted: Some("SELECT str[0:1]"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x, y : COALESCE(x, y, 0) + 1)",
                formatted: Some(
                    "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x, y: COALESCE(x, y, 0) + 1)",
                ),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT LIST_TRANSFORM(LIST_FILTER([0, 1, 2, 3, 4, 5], LAMBDA x : x % 2 = 0), LAMBDA y : y * y)",
                formatted: Some(
                    "SELECT LIST_TRANSFORM(LIST_FILTER([0, 1, 2, 3, 4, 5], LAMBDA x: x % 2 = 0), LAMBDA y: y * y)",
                ),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x : COALESCE(x, 0) + 1)",
                formatted: Some(
                    "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x: COALESCE(x, 0) + 1)",
                ),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT LIST_TRANSFORM(nbr, LAMBDA x : x + 1) FROM article AS a",
                formatted: Some("SELECT LIST_TRANSFORM(nbr, LAMBDA x: x + 1)\nFROM article AS a"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT JSON_EXTRACT(c, '$[*].id')[0:2]",
                formatted: Some("SELECT JSON_EXTRACT(c, '$[*].id')[0:2]"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT JSON_EXTRACT_STRING(c, '$[*].id')[0:2]",
                formatted: Some("SELECT JSON_EXTRACT_STRING(c, '$[*].id')[0:2]"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT [1, 2, 3][1 + 1:LENGTH([1, 2, 3]) + -1]",
                formatted: Some("SELECT [1, 2, 3][1 + 1:LENGTH([1, 2, 3]) + -1]"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT x::INT[3][3]",
                formatted: Some("SELECT x::INT[3][3]"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "SELECT ARRAY[[[1]]]::INT[1][1][1]",
                formatted: Some("SELECT ARRAY[[[1]]]::INT[1][1][1]"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
            Departure {
                sql: "WITH t1 AS (FROM (FROM t2 SELECT foo1, foo2)) FROM t1",
                formatted: Some(
                    "WITH t1 AS (\n  FROM (\n    FROM t2\n    SELECT\n      foo1,\n      foo2\n  )\n)\n\nFROM t1",
                ),
                reason: "SQLClean aligned against Polyglot's SELECT-first rewrite of the FROM-first query",
                receipt: None,
            },
            Departure {
                sql: "SELECT e: 1 + 2, f: len('asdf'), s: (SELECT 42)",
                formatted: Some("SELECT\n  e:1 + 2,\n  f:len('asdf'),\n  s:(\n    SELECT 42\n  )"),
                reason: "SQLClean aligned against Polyglot's path reading of the prefix alias and lost the line break after a comma",
                receipt: None,
            },
            Departure {
                sql: "SELECT (@col) + 1 FROM t",
                formatted: Some("SELECT\n  (\n    @col\n  )\n  + 1\nFROM t"),
                reason: "SQLClean aligned against Polyglot's ABS() rewrite of `@`",
                receipt: None,
            },
            Departure {
                sql: "SELECT (@-1) + 1",
                formatted: Some("SELECT\n  (\n    @-1\n  )\n  + 1"),
                reason: "SQLClean aligned against Polyglot's ABS() rewrite of `@`",
                receipt: None,
            },
            Departure {
                sql: "SELECT file[:256] FROM GLOB('*')",
                formatted: Some("SELECT file[:256]\nFROM GLOB('*')"),
                reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                receipt: None,
            },
        ],
    }];
    for test_case in test_cases {
        let outcome = corpus_outcome(
            FormatDialect::DuckDB,
            test_case.dialect,
            test_case.departures,
        );
        assert_eq!(
            (
                outcome.formatted,
                outcome.refused,
                outcome.departed,
                outcome.failures.len()
            ),
            (
                test_case.expected_formatted,
                test_case.expected_refused,
                test_case.departures.len(),
                0
            ),
            "{}: {}",
            test_case.description,
            outcome.failures.join("\n\n")
        );
    }
}
