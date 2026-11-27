//! Databricks statements converted to DuckDB, and the ones refused.

use crate::databricks::analysis::main::analyze::analyze;
use crate::databricks::conversion::models::{DatabricksType, Element};
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
            description: "^ is XOR in Databricks, so it becomes xor rather than DuckDB's power",
            query: "SELECT x1 ^ 2 AS r FROM input",
            expected_sql: &["xor(\"x1\", 2) AS \"r\""],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "LIKE ANY is an OR of LIKEs, and a simple CASE compares its operand",
            query: "SELECT s LIKE ANY ('a%', 'b') AS l, CASE x1 WHEN 1 THEN 'one' ELSE 'other' END AS c FROM input",
            expected_sql: &[
                "((\"s\" LIKE 'a%') OR (\"s\" LIKE 'b')) AS \"l\"",
                "CASE WHEN (\"x1\" = 1) THEN 'one' ELSE 'other' END AS \"c\"",
            ],
            expected_types: &[DatabricksType::Boolean, DatabricksType::String],
        },
        ConvertTestCase {
            description: "a literal cast to FLOAT converts only when no rounding happens",
            query: "SELECT CAST(-2.5 AS FLOAT) AS f FROM input",
            expected_sql: &["CAST(CAST(-2.5 AS DECIMAL(2, 1)) AS FLOAT) AS \"f\""],
            expected_types: &[DatabricksType::Float],
        },
        ConvertTestCase {
            description: "LEFT of a zero or negative length is '', where DuckDB counts from the end",
            query: "SELECT left(s, x1) AS l FROM input",
            expected_sql: &["WHEN \"x1\" <= 0 THEN ''"],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "TRY_MOD by -1 is 0, where DuckDB overflows on the smallest integer",
            query: "SELECT try_mod(x1, x1) AS m FROM input",
            expected_sql: &["CASE WHEN \"x1\" = -1 THEN \"x1\" * 0 ELSE try(\"x1\" % \"x1\") END"],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "TRY_DIVIDE by zero is NULL, where DuckDB's DOUBLE division gives NaN",
            query: "SELECT try_divide(x1, 0) AS q FROM input",
            expected_sql: &["CASE WHEN CAST(0 AS DOUBLE) = 0 THEN NULL ELSE"],
            expected_types: &[DatabricksType::Double],
        },
        ConvertTestCase {
            description: "GET is 0-based and NULL for a negative index, where DuckDB counts from the end",
            query: "SELECT get(array(1, 2147483648), x1) AS g, size(array()) AS n FROM input",
            expected_sql: &[
                "CASE WHEN \"x1\" < 0 THEN NULL ELSE [CAST(1 AS BIGINT), CAST(2147483648 AS BIGINT)][CAST(\"x1\" AS BIGINT) + 1] END AS \"g\"",
                "CAST(len([]) AS INTEGER) AS \"n\"",
            ],
            expected_types: &[DatabricksType::BigInt, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "an array result is written as Databricks's text, each element a string",
            query: "SELECT array(x1, NULL) AS a FROM input",
            expected_sql: &[
                "CAST(to_json(list_transform([\"x1\", NULL], lambda element: CAST(element AS VARCHAR))) AS VARCHAR) AS \"a\"",
            ],
            expected_types: &[DatabricksType::Array(Element::Int)],
        },
        ConvertTestCase {
            description: "text without a digit fails an integer cast in both engines, so it converts",
            query: "SELECT CAST('x' AS INT) AS i FROM input",
            expected_sql: &["CAST('x' AS INTEGER) AS \"i\""],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "division by a column raises on zero, where DuckDB's DOUBLE division gives infinity",
            query: "SELECT x1 / x1 AS q, x1 % -3 AS m FROM input",
            expected_sql: &[
                "WHEN CAST(\"x1\" AS DOUBLE) = 0 THEN CAST(0 % CAST(CAST(\"x1\" AS DOUBLE) AS INTEGER) AS DOUBLE)",
                "CASE WHEN -3 = -1 THEN \"x1\" * 0 ELSE \"x1\" % -3 END AS \"m\"",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "MOD of a decimal stays an exact decimal, typed as Databricks types it",
            query: "SELECT mod(CAST(x1 AS DECIMAL(12,2)), 3) AS m FROM input",
            expected_sql: &[
                "CAST(CAST(\"x1\" AS DECIMAL(12, 2)) % CAST(3 AS DECIMAL(10, 0)) AS DECIMAL(12, 2))",
            ],
            expected_types: &[DatabricksType::Decimal {
                precision: 12,
                scale: 2,
            }],
        },
        ConvertTestCase {
            description: "DAYOFWEEK counts Sunday as 1, one more than DuckDB's dayofweek",
            query: "SELECT dayofweek(DATE'2016-07-30') AS d FROM input",
            expected_sql: &["CAST(dayofweek(DATE '2016-07-30') + 1 AS INTEGER) AS \"d\""],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "COUNT_IF over only NULLs is 0, where DuckDB gives NULL; SUM of INT is BIGINT",
            query: "SELECT count_if(x1 > 0) AS c, sum(x1) AS t FROM input",
            expected_sql: &[
                "CAST(coalesce(count_if((\"x1\" > 0)), 0) AS BIGINT) AS \"c\"",
                "CAST(sum(\"x1\") AS BIGINT) AS \"t\"",
            ],
            expected_types: &[DatabricksType::BigInt, DatabricksType::BigInt],
        },
        ConvertTestCase {
            description: "DATE_ADD wraps its day count as an INT, as Databricks does",
            query: "SELECT date_add(DATE'2016-07-30', x1) AS d FROM input",
            expected_sql: &["% 4294967296 + 4294967296) % 4294967296 - 2147483648"],
            expected_types: &[DatabricksType::Date],
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
            description: "a date field of an integer, which Databricks would read as text",
            query: "SELECT dayofweek(x1) AS r FROM input",
            expected_reason: "DAYOFWEEK of (INT)",
        },
        RefuseTestCase {
            description: "case mapping differs on some characters",
            query: "SELECT upper(s) AS r FROM input",
            expected_reason: "UPPER",
        },
        RefuseTestCase {
            description: "a decimal quotient, which Databricks keeps exact and DOUBLE would round",
            query: "SELECT CAST(x1 AS DECIMAL(12,2)) / x1 AS r FROM input",
            expected_reason: "DECIMAL(12,2) div INT",
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
            expected_reason: "VOID result column",
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
            description: "0.1 is no binary fraction, so engines may round its FLOAT differently",
            query: "SELECT CAST(0.1 AS FLOAT) AS f FROM input",
            expected_reason: "FLOAT",
        },
        RefuseTestCase {
            description: "shifts wrap in Databricks and raise in DuckDB",
            query: "SELECT shiftleft(x1, 31) AS r FROM input",
            expected_reason: "SHIFTLEFT",
        },
        RefuseTestCase {
            description: "SHA2 of a length DuckDB has no digest for",
            query: "SELECT sha2(s, 384) AS h FROM input",
            expected_reason: "SHA2",
        },
        RefuseTestCase {
            description: "an array of strings, whose escapes the engines may write differently",
            query: "SELECT array(s) AS a FROM input",
            expected_reason: "ARRAY<STRING> result column",
        },
        RefuseTestCase {
            description: "text DuckDB reads as an infinite date, where Databricks raises",
            query: "SELECT CAST('inf' AS DATE) AS d FROM input",
            expected_reason: "DATE",
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
