use serde_json::Value;

use crate::format::tests::helpers::check_dialect_corpus;
use crate::format::tests::test_types;

const FORMAT_CORPUS: &str = include_str!("fixtures/format_corpus.json");

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [
        test_types::FormatCorpusTestCase {
            description: "DuckDB identity corpus",
            dialect: "duckdb",
            expected_formatted: 189,
            expected_refused: 0,
            departures: &[
                test_types::Departure {
                    sql: "SELECT str[0:1]",
                    formatted: Some("SELECT str[0:1]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x, y : COALESCE(x, y, 0) + 1)",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x, y: COALESCE(x, y, 0) + 1)",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM(LIST_FILTER([0, 1, 2, 3, 4, 5], LAMBDA x : x % 2 = 0), LAMBDA y : y * y)",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM(LIST_FILTER([0, 1, 2, 3, 4, 5], LAMBDA x: x % 2 = 0), LAMBDA y: y * y)",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x : COALESCE(x, 0) + 1)",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM([5, NULL, 6], LAMBDA x: COALESCE(x, 0) + 1)",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT LIST_TRANSFORM(nbr, LAMBDA x : x + 1) FROM article AS a",
                    formatted: Some(
                        "SELECT LIST_TRANSFORM(nbr, LAMBDA x: x + 1)\nFROM article AS a",
                    ),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT JSON_EXTRACT(c, '$[*].id')[0:2]",
                    formatted: Some("SELECT JSON_EXTRACT(c, '$[*].id')[0:2]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT JSON_EXTRACT_STRING(c, '$[*].id')[0:2]",
                    formatted: Some("SELECT JSON_EXTRACT_STRING(c, '$[*].id')[0:2]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT [1, 2, 3][1 + 1:LENGTH([1, 2, 3]) + -1]",
                    formatted: Some("SELECT [1, 2, 3][1 + 1:LENGTH([1, 2, 3]) + -1]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT x::INT[3][3]",
                    formatted: Some("SELECT x::INT[3][3]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT ARRAY[[[1]]]::INT[1][1][1]",
                    formatted: Some("SELECT ARRAY[[[1]]]::INT[1][1][1]"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "WITH t1 AS (FROM (FROM t2 SELECT foo1, foo2)) FROM t1",
                    formatted: Some(
                        "WITH t1 AS (\n  FROM (\n    FROM t2\n    SELECT\n      foo1,\n      foo2\n  )\n)\n\nFROM t1",
                    ),
                    reason: "SQLClean aligned against Polyglot's SELECT-first rewrite of the FROM-first query",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT e: 1 + 2, f: len('asdf'), s: (SELECT 42)",
                    formatted: Some(
                        "SELECT\n  e:1 + 2,\n  f:len('asdf'),\n  s:(\n    SELECT 42\n  )",
                    ),
                    reason: "SQLClean aligned against Polyglot's path reading of the prefix alias and lost the line break after a comma",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT (@col) + 1 FROM t",
                    formatted: Some("SELECT\n  (\n    @col\n  )\n  + 1\nFROM t"),
                    reason: "SQLClean aligned against Polyglot's ABS() rewrite of `@`",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT (@-1) + 1",
                    formatted: Some("SELECT\n  (\n    @-1\n  )\n  + 1"),
                    reason: "SQLClean aligned against Polyglot's ABS() rewrite of `@`",
                    receipt: None,
                },
                test_types::Departure {
                    sql: "SELECT file[:256] FROM GLOB('*')",
                    formatted: Some("SELECT file[:256]\nFROM GLOB('*')"),
                    reason: "SQLClean refused it: its Polyglot protection read a slice or lambda `:` as a path, or broke on a `::` array type",
                    receipt: None,
                },
            ],
        },
        test_types::FormatCorpusTestCase {
            description: "Snowflake identity corpus",
            dialect: "snowflake",
            expected_formatted: 427,
            expected_refused: 8,
            departures: &[
                test_types::Departure {
                    sql: "WITH t (SELECT 1 AS c) SELECT c FROM t",
                    formatted: None,
                    reason: "Snowflake rejects a CTE body without AS, so SQLQuack's parser refuses it",
                    receipt: Some("sf.query.with.select.k8b29420d51a4"),
                },
                test_types::Departure {
                    sql: "SELECT DATE_PART('month' FROM CAST('2024-04-08' AS DATE))",
                    formatted: None,
                    reason: "Snowflake rejects a quoted date part in the FROM form of DATE_PART, so SQLQuack's parser refuses it",
                    receipt: Some("sf.functions.date-part.date-part-from.k10b598e047b5"),
                },
                test_types::Departure {
                    sql: "WITH t AS (SELECT PARSE_JSON('{\"level1\": {\"level2\": {\"level3\": \"value\"}}}') AS data) SELECT data:     level1  : level2 : level3::VARIANT FROM t",
                    formatted: Some(
                        "WITH t AS (\n  SELECT PARSE_JSON('{\"level1\": {\"level2\": {\"level3\": \"value\"}}}') AS data\n)\n\nSELECT data:     level1  : level2 : level3::VARIANT\nFROM t",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, spaces included, so the item keeps its spacing; SQLClean respaced it",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT RPAD(TO_BINARY('Hi', 'UTF8'), 10, TO_BINARY('_', 'UTF8'))",
                    formatted: Some(
                        "SELECT\n  RPAD(TO_BINARY('Hi', 'UTF8'), 10, TO_BINARY('_', 'UTF8'))",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT TIMESTAMP_FROM_PARTS(CAST('2024-05-09' AS DATE), CAST('14:30:45' AS TIME))",
                    formatted: Some(
                        "SELECT\n  TIMESTAMP_FROM_PARTS(CAST('2024-05-09' AS DATE), CAST('14:30:45' AS TIME))",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT ARRAYS_ZIP([1, 2, 3], ['a', 'b', 'c'], [10, 20, 30])",
                    formatted: Some(
                        "SELECT\n  ARRAYS_ZIP([1, 2, 3], ['a', 'b', 'c'], [10, 20, 30])",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT AI_CLASSIFY('text', ['travel', 'cooking'], OBJECT_CONSTRUCT('output_mode', 'multi'))",
                    formatted: Some(
                        "SELECT\n  AI_CLASSIFY('text', ['travel', 'cooking'], OBJECT_CONSTRUCT('output_mode', 'multi'))",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT {fn DAYNAME('2022-5-13')}",
                    formatted: Some("SELECT {fn DAYNAME('2022-5-13')}"),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT {fn LOG(5)}",
                    formatted: Some("SELECT {fn LOG(5)}"),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT {fn CEILING(5.3)}",
                    formatted: Some("SELECT {fn CEILING(5.3)}"),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT TIME_SLICE(CAST('2024-05-09 08:50:57.891' AS TIMESTAMP), 1, 'HOUR', 'start')",
                    formatted: Some(
                        "SELECT\n  TIME_SLICE(CAST('2024-05-09 08:50:57.891' AS TIMESTAMP), 1, 'HOUR', 'start')",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT SEARCH(line, 'king', ANALYZER => 'UNICODE_ANALYZER', SEARCH_MODE => 'OR')",
                    formatted: Some(
                        "SELECT\n  SEARCH(line, 'king', ANALYZER => 'UNICODE_ANALYZER', SEARCH_MODE => 'OR')",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
                test_types::Departure {
                    sql: "SELECT SEARCH(line, 'king', SEARCH_MODE => 'AND', ANALYZER => 'PATTERN_ANALYZER')",
                    formatted: Some(
                        "SELECT\n  SEARCH(line, 'king', SEARCH_MODE => 'AND', ANALYZER => 'PATTERN_ANALYZER')",
                    ),
                    reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                    receipt: Some(
                        "cases/relational/unaliased_output_names.toml#unaliased_expressions",
                    ),
                },
            ],
        },
    ];
    let corpus: Vec<Value> = serde_json::from_str(FORMAT_CORPUS).unwrap_or_default();
    for test_case in test_cases {
        let outcome = check_dialect_corpus(&corpus, &test_case);
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
