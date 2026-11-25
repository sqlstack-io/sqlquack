use crate::helpers::{computed_macro_arguments, convert_streams, converted_first_value};
use crate::test_types::{ChainValueTestCase, ConvertedSqlTestCase, OperandCopiesTestCase};

#[test]
fn given_emitters_over_duckdb_macros_when_converting_then_duckdb_returns_snowflakes_value() {
    let test_cases = [
        ChainValueTestCase {
            description: "NULLIFZERO of a computed zero",
            sql: "SELECT COALESCE(TO_VARCHAR(NULLIFZERO(a + 1)), 'none') AS c FROM (SELECT -1 AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "NULLIFZERO of a computed value",
            sql: "SELECT COALESCE(TO_VARCHAR(NULLIFZERO(a + 1)), 'none') AS c FROM (SELECT 1 AS a)",
            expected_value: "2",
        },
        ChainValueTestCase {
            description: "nested NULLIFZERO",
            sql: "SELECT COALESCE(TO_VARCHAR(NULLIFZERO(NULLIFZERO(a + 1) * 2)), 'none') AS c FROM (SELECT 2 AS a)",
            expected_value: "6",
        },
        ChainValueTestCase {
            description: "nested NULLIFZERO of zero",
            sql: "SELECT COALESCE(TO_VARCHAR(NULLIFZERO(NULLIFZERO(a + 1) * 2)), 'none') AS c FROM (SELECT -1 AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "NULLIFZERO of a FLOAT",
            sql: "SELECT COALESCE(TO_VARCHAR(NULLIFZERO(CAST(a AS FLOAT) - 0.5)), 'none') AS c FROM (SELECT 1 AS a)",
            expected_value: "0.5",
        },
        ChainValueTestCase {
            description: "INITCAP of words",
            sql: "SELECT INITCAP(a) AS c FROM (SELECT 'hello wORLD' AS a)",
            expected_value: "Hello World",
        },
        ChainValueTestCase {
            description: "INITCAP of empty text",
            sql: "SELECT '[' || INITCAP(a) || ']' AS c FROM (SELECT '' AS a)",
            expected_value: "[]",
        },
        ChainValueTestCase {
            description: "INITCAP of NULL",
            sql: "SELECT COALESCE(INITCAP(a), 'none') AS c FROM (SELECT CAST(NULL AS VARCHAR) AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "JAROWINKLER_SIMILARITY of two words",
            sql: "SELECT TO_VARCHAR(JAROWINKLER_SIMILARITY(a, b)) AS c FROM (SELECT 'martha' AS a, 'MARHTA' AS b)",
            expected_value: "96",
        },
        ChainValueTestCase {
            description: "JAROWINKLER_SIMILARITY with empty text",
            sql: "SELECT TO_VARCHAR(JAROWINKLER_SIMILARITY(a, b)) AS c FROM (SELECT '' AS a, 'x' AS b)",
            expected_value: "0",
        },
        ChainValueTestCase {
            description: "ARRAY_TO_STRING of a VARIANT array",
            sql: "SELECT ARRAY_TO_STRING(PARSE_JSON(a), '-') AS c FROM (SELECT '[1,\"a\",{\"b\":2,\"a\":1}]' AS a)",
            expected_value: "1-a-{\"a\":1,\"b\":2}",
        },
        ChainValueTestCase {
            description: "ARRAY_TO_STRING of an empty VARIANT array",
            sql: "SELECT '[' || ARRAY_TO_STRING(PARSE_JSON(a), '-') || ']' AS c FROM (SELECT '[]' AS a)",
            expected_value: "[]",
        },
        ChainValueTestCase {
            description: "ARRAY_TO_STRING of a constructed array",
            sql: "SELECT ARRAY_TO_STRING(ARRAY_CONSTRUCT(a, 'y'), ',') AS c FROM (SELECT 'x' AS a)",
            expected_value: "x,y",
        },
        ChainValueTestCase {
            description: "ARRAY_TO_STRING of an empty constructed array",
            sql: "SELECT '[' || ARRAY_TO_STRING(ARRAY_CONSTRUCT(), ',') || ']' AS c",
            expected_value: "[]",
        },
        ChainValueTestCase {
            description: "a constructed array as text",
            sql: "SELECT TO_VARCHAR(ARRAY_CONSTRUCT(a, 2)) AS c FROM (SELECT 1 AS a)",
            expected_value: "[1,2]",
        },
        ChainValueTestCase {
            description: "an empty constructed array as text",
            sql: "SELECT TO_VARCHAR(ARRAY_CONSTRUCT()) AS c",
            expected_value: "[]",
        },
        ChainValueTestCase {
            description: "SPLIT_PART at part 0",
            sql: "SELECT SPLIT_PART(a, ',', 0) AS c FROM (SELECT 'x,y,z' AS a)",
            expected_value: "x",
        },
        ChainValueTestCase {
            description: "SPLIT_PART at part 2",
            sql: "SELECT SPLIT_PART(a, ',', 2) AS c FROM (SELECT 'x,y,z' AS a)",
            expected_value: "y",
        },
        ChainValueTestCase {
            description: "SPLIT_PART at part -1",
            sql: "SELECT SPLIT_PART(a, ',', -1) AS c FROM (SELECT 'x,y,z' AS a)",
            expected_value: "z",
        },
        ChainValueTestCase {
            description: "SPLIT_PART beyond the parts",
            sql: "SELECT '[' || SPLIT_PART(a, ',', 5) || ']' AS c FROM (SELECT 'x,y,z' AS a)",
            expected_value: "[]",
        },
        ChainValueTestCase {
            description: "SPLIT_PART of NULL",
            sql: "SELECT COALESCE(SPLIT_PART(a, ',', 2), 'none') AS c FROM (SELECT CAST(NULL AS VARCHAR) AS a)",
            expected_value: "none",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}

#[test]
fn given_emitters_over_duckdb_macros_when_converting_then_only_a_plain_window_reaches_a_repeating_macro()
 {
    let test_cases = [
        OperandCopiesTestCase {
            description: "NULLIFZERO of a computed operand",
            sql: "SELECT NULLIFZERO(a + 1) AS c FROM (SELECT 1 AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "NULLIFZERO nested three deep, which nullif doubled at each level",
            sql: "SELECT NULLIFZERO(NULLIFZERO(NULLIFZERO(a + 1) * 2) * 3) AS c FROM (SELECT 1 AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "UPPER's per-character lookup",
            sql: "SELECT UPPER(a) AS c FROM (SELECT 'x' AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "INITCAP's per-character casing",
            sql: "SELECT INITCAP(a) AS c FROM (SELECT 'x' AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "JAROWINKLER_SIMILARITY's encoding of each side",
            sql: "SELECT JAROWINKLER_SIMILARITY(a, b) AS c FROM (SELECT 'x' AS a, 'y' AS b)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "ARRAY_TO_STRING's writing of each VARIANT element",
            sql: "SELECT ARRAY_TO_STRING(PARSE_JSON(a), ',') AS c FROM (SELECT '[1]' AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "ARRAY_TO_STRING of a constructed array",
            sql: "SELECT ARRAY_TO_STRING(ARRAY_CONSTRUCT(a, 'y'), ',') AS c FROM (SELECT 'x' AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "NULLIFZERO of a window function over a computed argument",
            sql: "SELECT NULLIFZERO(SUM(a + 1) OVER ()) AS c FROM (SELECT 1 AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "NULLIFZERO of a window function over plain arguments, the one exception: \
                          DuckDB computes the window once however often nullif writes it",
            sql: "SELECT NULLIFZERO(SUM(a) OVER (PARTITION BY b)) AS c FROM (SELECT 1 AS a, 'x' AS b)",
            expected_copies: 1,
        },
        OperandCopiesTestCase {
            description: "SPLIT_PART at a literal part",
            sql: "SELECT SPLIT_PART(a, ',', 2) AS c FROM (SELECT 'x,y' AS a)",
            expected_copies: 0,
        },
    ];
    for test_case in test_cases {
        let copies = computed_macro_arguments(test_case.sql);

        assert_eq!(
            copies, test_case.expected_copies,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_ratio_to_report_when_converting_then_the_sql_is_byte_identical_to_0_24_0() {
    let test_cases = [
        ConvertedSqlTestCase {
            description: "RATIO_TO_REPORT over a partition",
            sql: "SELECT RATIO_TO_REPORT(x) OVER (PARTITION BY g) AS r FROM (SELECT CAST(1.5 AS NUMBER(10,2)) AS x, 'a' AS g)",
            expected_sql: r#"SELECT (CASE WHEN "__SQLQUACK_SUBQUERY_0"."X" IS NULL OR nullif(CAST(sum("__SQLQUACK_SUBQUERY_0"."X") OVER (PARTITION BY "__SQLQUACK_SUBQUERY_0"."G") AS DECIMAL(22,2)), 0) IS NULL THEN NULL WHEN nullif(CAST(sum("__SQLQUACK_SUBQUERY_0"."X") OVER (PARTITION BY "__SQLQUACK_SUBQUERY_0"."G") AS DECIMAL(22,2)), 0) = 0 THEN CAST(CAST('sqlquack:100051#484e0d07a67e: Division by zero' AS INTEGER) AS "NULL") ELSE CAST(CAST(((CAST(CAST("__SQLQUACK_SUBQUERY_0"."X" AS DECIMAL(38,2)) * 10000000000 AS HUGEINT) * 10) // CAST(CAST(nullif(CAST(sum("__SQLQUACK_SUBQUERY_0"."X") OVER (PARTITION BY "__SQLQUACK_SUBQUERY_0"."G") AS DECIMAL(22,2)), 0) AS DECIMAL(38,2)) * 100 AS HUGEINT)) AS DECIMAL(38,0)) * 0.000000001 AS DECIMAL(18,8)) END) AS "R" FROM (SELECT CAST(1.5 AS DECIMAL(10,2)) AS "X", 'a' AS "G") AS "__SQLQUACK_SUBQUERY_0"("X", "G")"#,
        },
        ConvertedSqlTestCase {
            description: "RATIO_TO_REPORT over the whole result",
            sql: "SELECT RATIO_TO_REPORT(x) OVER () AS r FROM (SELECT CAST(1.5 AS NUMBER(10,2)) AS x)",
            expected_sql: r#"SELECT (CASE WHEN "__SQLQUACK_SUBQUERY_0"."X" IS NULL OR nullif(CAST(sum("__SQLQUACK_SUBQUERY_0"."X") OVER () AS DECIMAL(22,2)), 0) IS NULL THEN NULL WHEN nullif(CAST(sum("__SQLQUACK_SUBQUERY_0"."X") OVER () AS DECIMAL(22,2)), 0) = 0 THEN CAST(CAST('sqlquack:100051#35fe8ffca89c: Division by zero' AS INTEGER) AS "NULL") ELSE CAST(CAST(((CAST(CAST("__SQLQUACK_SUBQUERY_0"."X" AS DECIMAL(38,2)) * 10000000000 AS HUGEINT) * 10) // CAST(CAST(nullif(CAST(sum("__SQLQUACK_SUBQUERY_0"."X") OVER () AS DECIMAL(22,2)), 0) AS DECIMAL(38,2)) * 100 AS HUGEINT)) AS DECIMAL(38,0)) * 0.000000001 AS DECIMAL(18,8)) END) AS "R" FROM (SELECT CAST(1.5 AS DECIMAL(10,2)) AS "X") AS "__SQLQUACK_SUBQUERY_0"("X")"#,
        },
    ];
    for test_case in test_cases {
        let (converted, _) = convert_streams(test_case.sql, &["--no-config", "--compact"]);

        assert_eq!(
            converted.trim_end(),
            test_case.expected_sql,
            "{}",
            test_case.description
        );
    }
}
