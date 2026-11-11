use crate::bigquery::profile::tests::test_types;
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::corpus_outcome;
use crate::format::tests::test_types::Departure;

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [test_types::FormatCorpusTestCase {
        description: "BigQuery identity corpus",
        dialect: "bigquery",
        expected_formatted: 161,
        expected_refused: 7,
        departures: &[
            Departure {
                sql: "SELECT * FROM x.*",
                formatted: None,
                reason: "BigQuery rejects it: `Syntax error: Unexpected \"*\" at [1:17]`; SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/bigquery/profile/tests/fixtures/bigquery_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT * FROM x.y*",
                formatted: None,
                reason: "BigQuery rejects it: a wildcard table needs backquotes (`Syntax error: Expected end of input but got \"*\" at [1:18]`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/bigquery/profile/tests/fixtures/bigquery_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT 1 AS hash",
                formatted: None,
                reason: "BigQuery rejects it: HASH is a reserved keyword (`Syntax error: Unexpected keyword HASH at [1:13]`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/bigquery/profile/tests/fixtures/bigquery_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT 1 AS at",
                formatted: None,
                reason: "BigQuery rejects it: AT is a reserved keyword (`Syntax error: Unexpected keyword AT at [1:13]`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/bigquery/profile/tests/fixtures/bigquery_receipts.jsonl",
                ),
            },
            Departure {
                sql: "WITH cte(c) AS (SELECT * FROM t) SELECT * FROM cte",
                formatted: None,
                reason: "BigQuery rejects it: GoogleSQL has no CTE column list (`Syntax error: Expected \")\" but got identifier \"c\" at [1:10]`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/bigquery/profile/tests/fixtures/bigquery_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT * FROM t AS t(c1, c2)",
                formatted: None,
                reason: "BigQuery rejects it: GoogleSQL has no alias column list (`Syntax error: Expected end of input but got \"(\" at [1:21]`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/bigquery/profile/tests/fixtures/bigquery_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT ARRAY(SELECT AS STRUCT 1 a, 2 b)",
                formatted: Some(
                    "SELECT\n  ARRAY(\n    SELECT AS STRUCT\n      1 a,\n      2 b\n  )",
                ),
                reason: "SQLClean's Polyglot printer left a subquery call argument on the call's line; SQLQuack lays it out as any other subquery, as for PostgreSQL's ARRAY(SELECT ...)",
                receipt: None,
            },
            Departure {
                sql: "SELECT STRUCT<STRING>((SELECT 'foo')).*",
                formatted: Some(
                    "SELECT\n  STRUCT<STRING> (\n    (\n      SELECT 'foo'\n    )\n  ).*",
                ),
                reason: "SQLClean's Polyglot printer put a space before `.*`; SQLQuack keeps the field access touching its operand",
                receipt: None,
            },
            Departure {
                sql: "select array_contains([1, 2, 3], 1)",
                formatted: Some("SELECT ARRAY_CONTAINS([1, 2, 3], 1)"),
                reason: "SQLQuack upper-cases a called function name, as in every dialect, and BigQuery function names are case-insensitive; SQLClean kept the authored case",
                receipt: None,
            },
        ],
    }];
    for test_case in test_cases {
        let outcome = corpus_outcome(
            FormatDialect::BigQuery,
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
