//! Databricks statements converted to DuckDB, and the ones refused.

use crate::databricks::analysis::main::analyze::analyze;
use crate::databricks::conversion::models::DatabricksType;
use crate::databricks::conversion::tests::test_types::{ConvertTestCase, RefuseTestCase};

const FIXTURE: &str = "WITH input AS (SELECT * FROM VALUES (1, CAST(-7 AS INT), CAST('a' AS STRING)), \
     (2, CAST(NULL AS INT), CAST(NULL AS STRING)) AS input(ordinal, x1, s)) ";

#[test]
fn given_a_query_over_the_fixture_when_converted_then_duckdb_sql_keeps_databricks_types() {
    let test_cases = [
        ConvertTestCase {
            description: "the fixture as a VALUES list, a same-named function, NULLs first ascending",
            query: "SELECT ABS ( x1 ) AS r FROM input ORDER BY ordinal",
            expected_sql: &[
                "WITH \"input\" AS (SELECT * FROM (VALUES (1, CAST(-7 AS INTEGER), CAST('a' AS VARCHAR)), \
                 (2, CAST(NULL AS INTEGER), CAST(NULL AS VARCHAR))) AS \"input\"(\"ordinal\", \"x1\", \"s\")) \
                 SELECT abs(\"x1\") AS \"r\" FROM \"input\" ORDER BY \"ordinal\" ASC NULLS FIRST",
            ],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "decimal literals keep their digits, a negated 2^31 is BIGINT, 1e3 is DOUBLE",
            query: "SELECT 2.50 AS d, 0.01 AS e, 2147483648 AS b, -2147483648 AS n, 1e3 AS f FROM input",
            expected_sql: &[
                "CAST(2.50 AS DECIMAL(3, 2)) AS \"d\"",
                "CAST(0.01 AS DECIMAL(2, 2)) AS \"e\"",
                "CAST(-2147483648 AS BIGINT) AS \"n\"",
            ],
            expected_types: &[
                DatabricksType::Decimal {
                    precision: 3,
                    scale: 2,
                },
                DatabricksType::Decimal {
                    precision: 2,
                    scale: 2,
                },
                DatabricksType::BigInt,
                DatabricksType::BigInt,
                DatabricksType::Double,
            ],
        },
        ConvertTestCase {
            description: "integer division is DOUBLE, modulo keeps INT, NULLs last descending",
            query: "SELECT x1 / 2 AS q, x1 % 3 AS m FROM input ORDER BY q DESC",
            expected_sql: &[
                "(CAST(\"x1\" AS DOUBLE) / CAST(2 AS DOUBLE)) AS \"q\"",
                "ORDER BY \"q\" DESC NULLS LAST",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "trim is told to trim only spaces, as Databricks does",
            query: "SELECT trim(s) AS t FROM input",
            expected_sql: &["trim(\"s\", ' ') AS \"t\""],
            expected_types: &[DatabricksType::String],
        },
    ];
    for test_case in test_cases {
        let converted = analyze(&format!("{FIXTURE}{}", test_case.query))
            .unwrap_or_else(|refusal| panic!("{}: {}", test_case.description, refusal.reason));
        for expected in test_case.expected_sql {
            assert!(
                converted.sql.contains(expected),
                "{}: {}",
                test_case.description,
                converted.sql
            );
        }
        let types: Vec<DatabricksType> = converted
            .columns
            .iter()
            .map(|(_, data_type)| *data_type)
            .collect();
        assert_eq!(types, test_case.expected_types, "{}", test_case.description);
    }
}

#[test]
fn given_a_hazard_or_unknown_construct_when_converted_then_it_is_refused() {
    let test_cases = [
        RefuseTestCase {
            description: "^ is XOR in Databricks and a power in DuckDB",
            query: "SELECT x1 ^ 2 AS r FROM input",
            expected_reason: "bitwise_xor",
        },
        RefuseTestCase {
            description: "dayofweek numbers days from Sunday = 1",
            query: "SELECT dayofweek(x1) AS r FROM input",
            expected_reason: "DAYOFWEEK",
        },
        RefuseTestCase {
            description: "case mapping differs on some characters",
            query: "SELECT upper(s) AS r FROM input",
            expected_reason: "UPPER",
        },
        RefuseTestCase {
            description: "division by a column may divide by zero",
            query: "SELECT x1 / x1 AS r FROM input",
            expected_reason: "by a constant",
        },
        RefuseTestCase {
            description: "modulo by a negative constant",
            query: "SELECT x1 % -3 AS r FROM input",
            expected_reason: "by a constant",
        },
        RefuseTestCase {
            description: "a LIKE pattern holding Databricks's escape",
            query: "SELECT s LIKE 'a\\\\%' AS r FROM input",
            expected_reason: "backslash",
        },
        RefuseTestCase {
            description: "an unaliased call, whose name Databricks generates",
            query: "SELECT abs(x1) FROM input",
            expected_reason: "unaliased",
        },
        RefuseTestCase {
            description: "a column spelled in another case, which names the result differently",
            query: "SELECT X1 FROM input",
            expected_reason: "spelled unlike",
        },
        RefuseTestCase {
            description: "a NULL result column, VOID in Databricks",
            query: "SELECT NULL AS r FROM input",
            expected_reason: "NULL result column",
        },
        RefuseTestCase {
            description: "ORDER BY a position",
            query: "SELECT x1 AS r FROM input ORDER BY 1",
            expected_reason: "position",
        },
        RefuseTestCase {
            description: "a string compared with an integer",
            query: "SELECT s = x1 AS r FROM input",
            expected_reason: "compares",
        },
        RefuseTestCase {
            description: "an OFFSET no rule reads",
            query: "SELECT x1 AS r FROM input LIMIT 1 OFFSET 1",
            expected_reason: "offset",
        },
    ];
    for test_case in test_cases {
        let reason = analyze(&format!("{FIXTURE}{}", test_case.query))
            .expect_err(test_case.description)
            .reason;
        assert!(
            reason.contains(test_case.expected_reason),
            "{}: {reason}",
            test_case.description
        );
    }
}
