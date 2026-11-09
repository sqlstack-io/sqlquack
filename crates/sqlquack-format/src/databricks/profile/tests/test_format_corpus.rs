use crate::databricks::profile::tests::test_types;
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::corpus_outcome;
use crate::format::tests::test_types::Departure;

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [test_types::FormatCorpusTestCase {
        description: "Databricks identity corpus",
        dialect: "databricks",
        expected_formatted: 61,
        expected_refused: 1,
        departures: &[
            Departure {
                sql: "SELECT ${x} FROM ${y} WHERE ${z} > 1",
                formatted: None,
                reason: "`${x}` is a variable substitution that Databricks replaces before parsing, so its grammar has no such token and the native parser rejects it; SQLClean formatted it",
                receipt: None,
            },
            Departure {
                sql: "WITH t AS (SELECT '{\"x-y\": \"z\"}' AS c) SELECT get_json_object(c, '$.x-y') FROM t",
                formatted: Some(
                    "WITH t AS (\n  SELECT '{\"x-y\": \"z\"}' AS c\n)\n\nSELECT GET_JSON_OBJECT(c, '$.x-y')\nFROM t",
                ),
                reason: "SQLClean kept the call's authored case because its Polyglot oracle rewrites the JSON path `'$.x-y'` as `'$[\"x-y\"]'`, as SQLGlot does; SQLQuack upper-cases a called name in every dialect",
                receipt: None,
            },
        ],
    }];
    for test_case in test_cases {
        let outcome = corpus_outcome(
            FormatDialect::Databricks,
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
