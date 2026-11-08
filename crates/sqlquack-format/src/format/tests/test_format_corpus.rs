use serde_json::Value;

use crate::format::tests::helpers::check_dialect_corpus;
use crate::format::tests::test_types;

const FORMAT_CORPUS: &str = include_str!("fixtures/format_corpus.json");

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [test_types::FormatCorpusTestCase {
        description: "Snowflake identity corpus",
        dialect: "snowflake",
        expected_formatted: 426,
        expected_refused: 9,
        departures: &[
            test_types::Departure {
                sql: "WITH t (SELECT 1 AS c) SELECT c FROM t",
                formatted: None,
                reason: "Snowflake rejects a CTE body without AS, so SQLQuack's parser refuses it",
                receipt: "sf.query.with.select.k8b29420d51a4",
            },
            test_types::Departure {
                sql: "SELECT DATE_PART('month' FROM CAST('2024-04-08' AS DATE))",
                formatted: None,
                reason: "Snowflake rejects a quoted date part in the FROM form of DATE_PART, so SQLQuack's parser refuses it",
                receipt: "sf.functions.date-part.date-part-from.k10b598e047b5",
            },
            test_types::Departure {
                sql: "SELECT * FROM t, UNNEST(x) WITH ORDINALITY",
                formatted: None,
                reason: "Snowflake has no UNNEST, so SQLQuack's parser refuses it",
                receipt: "sf.query.format-probe.k47586a1689b7",
            },
            test_types::Departure {
                sql: "WITH t AS (SELECT PARSE_JSON('{\"level1\": {\"level2\": {\"level3\": \"value\"}}}') AS data) SELECT data:     level1  : level2 : level3::VARIANT FROM t",
                formatted: Some(
                    "WITH t AS (\n  SELECT PARSE_JSON('{\"level1\": {\"level2\": {\"level3\": \"value\"}}}') AS data\n)\n\nSELECT data:     level1  : level2 : level3::VARIANT\nFROM t",
                ),
                reason: "Snowflake names an unaliased select item by its text, spaces included, so the item keeps its spacing; SQLClean respaced it",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT RPAD(TO_BINARY('Hi', 'UTF8'), 10, TO_BINARY('_', 'UTF8'))",
                formatted: Some(
                    "SELECT\n  RPAD(TO_BINARY('Hi', 'UTF8'), 10, TO_BINARY('_', 'UTF8'))",
                ),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT TIMESTAMP_FROM_PARTS(CAST('2024-05-09' AS DATE), CAST('14:30:45' AS TIME))",
                formatted: Some(
                    "SELECT\n  TIMESTAMP_FROM_PARTS(CAST('2024-05-09' AS DATE), CAST('14:30:45' AS TIME))",
                ),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT ARRAYS_ZIP([1, 2, 3], ['a', 'b', 'c'], [10, 20, 30])",
                formatted: Some("SELECT\n  ARRAYS_ZIP([1, 2, 3], ['a', 'b', 'c'], [10, 20, 30])"),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT AI_CLASSIFY('text', ['travel', 'cooking'], OBJECT_CONSTRUCT('output_mode', 'multi'))",
                formatted: Some(
                    "SELECT\n  AI_CLASSIFY('text', ['travel', 'cooking'], OBJECT_CONSTRUCT('output_mode', 'multi'))",
                ),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT {fn DAYNAME('2022-5-13')}",
                formatted: Some("SELECT {fn DAYNAME('2022-5-13')}"),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT {fn LOG(5)}",
                formatted: Some("SELECT {fn LOG(5)}"),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT {fn CEILING(5.3)}",
                formatted: Some("SELECT {fn CEILING(5.3)}"),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean broke a line inside its `{fn ...}` escape",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT TIME_SLICE(CAST('2024-05-09 08:50:57.891' AS TIMESTAMP), 1, 'HOUR', 'start')",
                formatted: Some(
                    "SELECT\n  TIME_SLICE(CAST('2024-05-09 08:50:57.891' AS TIMESTAMP), 1, 'HOUR', 'start')",
                ),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT SEARCH(line, 'king', ANALYZER => 'UNICODE_ANALYZER', SEARCH_MODE => 'OR')",
                formatted: Some(
                    "SELECT\n  SEARCH(line, 'king', ANALYZER => 'UNICODE_ANALYZER', SEARCH_MODE => 'OR')",
                ),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
            test_types::Departure {
                sql: "SELECT SEARCH(line, 'king', SEARCH_MODE => 'AND', ANALYZER => 'PATTERN_ANALYZER')",
                formatted: Some(
                    "SELECT\n  SEARCH(line, 'king', SEARCH_MODE => 'AND', ANALYZER => 'PATTERN_ANALYZER')",
                ),
                reason: "Snowflake names an unaliased select item by its text, line breaks included, so the item stays on one line; SQLClean split its arguments",
                receipt: "cases/relational/unaliased_output_names.toml#unaliased_expressions",
            },
        ],
    }];
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
