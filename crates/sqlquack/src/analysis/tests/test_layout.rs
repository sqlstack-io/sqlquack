use sqlquack_blocks::layout::main::checked_layout::checked_layout;
use sqlquack_blocks::layout::main::layout_sql::layout_sql;
use sqlquack_blocks::macros::main::all_helper_statements::all_helper_statements;
use sqlquack_format::FormatDialect;

use crate::analysis::tests::helpers::laid_out_creation_failure;
use crate::analysis::tests::test_types::{
    LayoutCreationTestCase, LayoutFallbackTestCase, LayoutTestCase,
};

#[test]
fn given_converted_sql_when_laid_out_then_long_lines_break_and_only_whitespace_and_case_change() {
    let test_cases = [
        LayoutTestCase {
            description: "a short query stays on one line",
            sql: r#"SELECT CAST(1 AS DECIMAL(1,0)) AS "1""#,
            expected_layout: r#"SELECT CAST(1 AS DECIMAL(1, 0)) AS "1""#,
        },
        LayoutTestCase {
            description: "a wide decimal division breaks its HUGEINT casts and its `//` over indented lines",
            sql: r#"SELECT "ORDERS"."ORDER_ID" AS "ORDER_ID", CAST(CAST((sign(CAST(CAST("ORDERS"."TOTAL" AS DECIMAL(38,2)) * 100000000 AS HUGEINT)) * ((2 * abs(CAST(CAST("ORDERS"."TOTAL" AS DECIMAL(38,2)) * 100000000 AS HUGEINT)) + 3) // 6)) AS DECIMAL(38,0)) * 0.00000001 AS DECIMAL(18,8)) AS "THIRD" FROM "ORDERS" AS "ORDERS""#,
            expected_layout: r#"SELECT
    "ORDERS"."ORDER_ID" AS "ORDER_ID",
    CAST(
        CAST(
            (
                SIGN(CAST(CAST("ORDERS"."TOTAL" AS DECIMAL(38, 2)) * 100000000 AS HUGEINT))
                * (
                    (
                        2
                        * ABS(CAST(CAST("ORDERS"."TOTAL" AS DECIMAL(38, 2)) * 100000000 AS HUGEINT))
                        + 3
                    )
                    // 6
                )
            ) AS DECIMAL(38, 0)
        )
        * 0.00000001 AS DECIMAL(18, 8)
    ) AS "THIRD"
FROM "ORDERS" AS "ORDERS""#,
        },
        LayoutTestCase {
            description: "a long CASE in a lambda breaks at its branches",
            sql: r#"SELECT list_transform([1], lambda x: CASE WHEN abs(x) > 9007199254740992 THEN CAST('a long raise message' AS INTEGER) ELSE x END)[1] AS "E" FROM "T" AS "T""#,
            expected_layout: r#"SELECT
    LIST_TRANSFORM(
        [1],
        LAMBDA x: CASE
            WHEN ABS(x) > 9007199254740992
            THEN CAST('a long raise message' AS INTEGER)
            ELSE x
        END
    )[1] AS "E"
FROM "T" AS "T""#,
        },
        LayoutTestCase {
            description: "a macro keeps CREATE OR REPLACE together, and a lambda body that only starts with a parenthesis stays inline",
            sql: r#"CREATE OR REPLACE MACRO sqlquack.quotient_v1(quack_dividend, quack_divisor, quack_power) AS list_transform([quack_dividend % quack_divisor], lambda quack_rest: (quack_dividend // quack_divisor) * quack_power + quack_rest // quack_divisor)[1]"#,
            expected_layout: r#"CREATE OR REPLACE MACRO sqlquack.quotient_v1(
    quack_dividend,
    quack_divisor,
    quack_power
) AS list_transform(
    [quack_dividend % quack_divisor],
    LAMBDA quack_rest: (quack_dividend // quack_divisor) * quack_power + quack_rest // quack_divisor
)[1]"#,
        },
        LayoutTestCase {
            description: "keywords in strings and quoted names, and doubled spaces in a dollar-quoted string, are kept",
            sql: r#"SELECT DISTINCT * EXCEPT ("B"), 'x FROM y, z' AS "WHERE, FROM", $$a  FROM  b,  WHERE c$$ AS "TEXT" FROM "T" AS "T" WHERE ("T"."A" IS DISTINCT FROM "T"."B")"#,
            expected_layout: r#"SELECT DISTINCT
    * EXCEPT ("B"),
    'x FROM y, z' AS "WHERE, FROM",
    $$a  FROM  b,  WHERE c$$ AS "TEXT"
FROM "T" AS "T"
WHERE ("T"."A" IS DISTINCT FROM "T"."B")"#,
        },
    ];
    for test_case in test_cases {
        let laid_out = checked_layout(test_case.sql, FormatDialect::DuckDB);
        assert_eq!(
            laid_out.as_deref(),
            Ok(test_case.expected_layout),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_the_formatter_refuses_when_laid_out_then_the_single_line_is_kept() {
    let test_cases = [LayoutFallbackTestCase {
        description: "SQL that does not parse",
        sql: r#"SELECT ("T"."A" + FROM "T" AS "T""#,
        expected_reason: "the formatter refused it",
    }];
    for test_case in test_cases {
        let refusal =
            checked_layout(test_case.sql, FormatDialect::DuckDB).expect_err(test_case.description);
        assert!(
            refusal.contains(test_case.expected_reason),
            "{}: {refusal}",
            test_case.description
        );
        assert_eq!(
            layout_sql(test_case.sql, FormatDialect::DuckDB),
            test_case.sql,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_every_helper_macro_when_laid_out_then_duckdb_creates_it() {
    let test_cases = [LayoutCreationTestCase {
        description: "every helper laid out over lines, as convert prints it, is created",
        expected_failures: vec![],
    }];
    for test_case in test_cases {
        let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
        let failures: Vec<String> = all_helper_statements()
            .iter()
            .filter_map(|statement| laid_out_creation_failure(&connection, statement).err())
            .collect();
        assert_eq!(
            failures, test_case.expected_failures,
            "{}",
            test_case.description
        );
    }
}
