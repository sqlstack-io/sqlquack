use crate::format::models::FormatDialect;
use crate::format::tests::helpers::corpus_outcome;
use crate::format::tests::test_types::Departure;
use crate::sqlserver::profile::tests::test_types;

#[test]
fn given_dialect_corpus_when_formatting_then_output_layout_and_refusals_are_stable() {
    let test_cases = [test_types::FormatCorpusTestCase {
        description: "SQL Server identity corpus",
        dialect: "tsql",
        expected_formatted: 68,
        expected_refused: 7,
        departures: &[
            Departure {
                sql: "SELECT val FROM (VALUES ((TRUE), (FALSE), (NULL))) AS t(val)",
                formatted: Some(
                    "SELECT val\nFROM (\n  VALUES\n    ((\n  TRUE\n), (\n  FALSE\n), (\n  NULL\n))\n) AS t(val)",
                ),
                reason: "The shared layout of a parenthesised value in a VALUES row, the same as Snowflake's; SQLClean broke after the values instead",
                receipt: None,
            },
            Departure {
                sql: "WITH t1 AS (SELECT 1 AS a), t2 AS (SELECT 1 AS a) SELECT COUNT(*) FROM (SELECT TOP 10 a FROM t1 UNION ALL SELECT TOP 10 a FROM t2 ORDER BY a DESC) AS t",
                formatted: Some(
                    "WITH t1 AS (\n  SELECT 1 AS a\n),\n\nt2 AS (\n  SELECT 1 AS a\n)\n\nSELECT COUNT(*)\nFROM (\n  SELECT TOP 10 a\n  FROM t1\n  UNION ALL\n  SELECT TOP 10 a\n  FROM t2\n  ORDER BY a DESC\n) AS t",
                ),
                reason: "A set operation's ORDER BY sits at the depth of its set operation; SQLClean outdented it to the enclosing parenthesis",
                receipt: None,
            },
            Departure {
                sql: "SELECT * FROM t WHERE NOT c",
                formatted: None,
                reason: "SQL Server rejects it: `An expression of non-boolean type specified in a context where a condition is expected, near 'c'.` (4145); SQLQuack's parser refuses it (`An expression of non-boolean type specified in a context where a condition is expected.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT TOP 10 PERCENT",
                formatted: None,
                reason: "SQL Server rejects it: `Incorrect syntax near 'PERCENT'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near the end of the input.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT TOP 10 PERCENT WITH TIES",
                formatted: None,
                reason: "SQL Server rejects it: `Incorrect syntax near 'TIES'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near the end of the input.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME FROM c TO d",
                formatted: None,
                reason: "SQL Server rejects it: `Incorrect syntax near 'c'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near 'c'.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME BETWEEN c AND d",
                formatted: None,
                reason: "SQL Server rejects it: `Incorrect syntax near 'c'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near 'c'.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME CONTAINED IN (c, d)",
                formatted: None,
                reason: "SQL Server rejects it: `Incorrect syntax near 'c'.` (102); SQLQuack's parser refuses it (`Incorrect syntax near 'c'.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
            Departure {
                sql: "SELECT NEXT VALUE FOR db.schema.sequence_name OVER (ORDER BY foo), col",
                formatted: None,
                reason: "SQL Server rejects it: `Incorrect syntax near the keyword 'schema'.` (156); SQLQuack's parser refuses it (`Incorrect syntax near 'schema'.`); SQLClean formatted it",
                receipt: Some(
                    "crates/sqlquack-format/src/sqlserver/profile/tests/fixtures/sqlserver_receipts.jsonl",
                ),
            },
        ],
    }];
    for test_case in test_cases {
        let outcome = corpus_outcome(
            FormatDialect::SqlServer,
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
