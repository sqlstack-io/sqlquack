use crate::layout::main::checked_layout::checked_layout;
use crate::layout::main::layout_sql::layout_sql;
use crate::layout::tests::test_types::{FallbackTestCase, LayoutTestCase};

#[test]
fn given_converted_sql_when_laid_out_then_clauses_items_and_long_cases_break_by_depth() {
    let test_cases = [
        LayoutTestCase {
            description: "a short query stays on one line",
            sql: r#"SELECT CAST(1 AS DECIMAL(1,0)) AS "1""#,
            expected_layout: r#"SELECT CAST(1 AS DECIMAL(1,0)) AS "1""#,
        },
        LayoutTestCase {
            description: "a common table expression, a join, a long CASE and a short subquery",
            sql: r#"WITH "X" AS (SELECT "O"."CUST" AS "CUST", sum("O"."AMT") AS "T" FROM "ORDERS" AS "O" GROUP BY "O"."CUST" HAVING (sum("O"."AMT") > 10)) SELECT "C"."NAME" AS "NAME", (CASE WHEN ("X"."T" > 100) THEN 'big' WHEN ("X"."T" > 50) THEN 'mid' ELSE 'small' END) AS "SZ" FROM "X" AS "X" LEFT JOIN "CUSTOMERS" AS "C" ON ("C"."NAME" = "X"."CUST") WHERE ("X"."T" IN (SELECT "O"."AMT" FROM "ORDERS" AS "O")) ORDER BY "X"."T" DESC NULLS FIRST LIMIT 5"#,
            expected_layout: r#"WITH "X" AS (
    SELECT
        "O"."CUST" AS "CUST",
        sum("O"."AMT") AS "T"
    FROM "ORDERS" AS "O"
    GROUP BY "O"."CUST"
    HAVING (sum("O"."AMT") > 10)
)
SELECT
    "C"."NAME" AS "NAME",
    (CASE
        WHEN ("X"."T" > 100) THEN 'big'
        WHEN ("X"."T" > 50) THEN 'mid'
        ELSE 'small'
    END) AS "SZ"
FROM "X" AS "X"
LEFT JOIN "CUSTOMERS" AS "C" ON ("C"."NAME" = "X"."CUST")
WHERE ("X"."T" IN (SELECT "O"."AMT" FROM "ORDERS" AS "O"))
ORDER BY "X"."T" DESC NULLS FIRST
LIMIT 5"#,
        },
        LayoutTestCase {
            description: "a set operation stands on its own line, and a short branch stays inline",
            sql: r#"(SELECT "T"."ID" AS "ID", "T"."NAME" AS "NAME" FROM "T" AS "T" WHERE ("T"."ID" > 1)) UNION ALL (SELECT "U"."ID" AS "ID", "U"."NAME" AS "NAME" FROM "U" AS "U")"#,
            expected_layout: r#"(
    SELECT
        "T"."ID" AS "ID",
        "T"."NAME" AS "NAME"
    FROM "T" AS "T"
    WHERE ("T"."ID" > 1)
)
UNION ALL (SELECT "U"."ID" AS "ID", "U"."NAME" AS "NAME" FROM "U" AS "U")"#,
        },
        LayoutTestCase {
            description: "a long CASE inside a function call stays inline",
            sql: r#"SELECT list_transform([1], lambda x: CASE WHEN abs(x) > 9007199254740992 THEN CAST('a long raise message' AS INTEGER) ELSE x END)[1] AS "E" FROM "T" AS "T""#,
            expected_layout: r#"SELECT
    list_transform([1], lambda x: CASE WHEN abs(x) > 9007199254740992 THEN CAST('a long raise message' AS INTEGER) ELSE x END)[1] AS "E"
FROM "T" AS "T""#,
        },
        LayoutTestCase {
            description: "keywords in strings, quoted names, IS DISTINCT FROM and * EXCEPT do not break",
            sql: r#"SELECT DISTINCT * EXCEPT ("B"), 'x FROM y, z' AS "WHERE, FROM" FROM "T" AS "T" WHERE ("T"."A" IS DISTINCT FROM "T"."B")"#,
            expected_layout: r#"SELECT DISTINCT
    * EXCEPT ("B"),
    'x FROM y, z' AS "WHERE, FROM"
FROM "T" AS "T"
WHERE ("T"."A" IS DISTINCT FROM "T"."B")"#,
        },
    ];
    for test_case in test_cases {
        let laid_out = checked_layout(test_case.sql);
        assert_eq!(
            laid_out.as_deref(),
            Ok(test_case.expected_layout),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_text_the_layout_does_not_model_when_laid_out_then_the_single_line_is_kept() {
    let test_cases = [
        FallbackTestCase {
            description: "a dollar-quoted string holding clause words and doubled spaces",
            sql: r#"SELECT $$a  FROM  b,  WHERE c$$ AS "TEXT", "T"."A" AS "A" FROM "T" AS "T" WHERE ("T"."A" > 1)"#,
            expected_reason: "a `$`",
        },
        FallbackTestCase {
            description: "an escape string with an escaped quote and doubled spaces",
            sql: r#"SELECT E'it\'s  FROM  here' AS "TEXT", "T"."A" AS "A" FROM "T" AS "T" WHERE ("T"."A" > 1)"#,
            expected_reason: "a prefixed string",
        },
        FallbackTestCase {
            description: "a nested comment holding a clause word",
            sql: r#"SELECT /* a /* b */ FROM  c */ "T"."A" AS "A", "T"."B" AS "B" FROM "T" AS "T" WHERE ("T"."A" > 1)"#,
            expected_reason: "an unclosed or nested comment",
        },
    ];
    for test_case in test_cases {
        let refusal = checked_layout(test_case.sql).expect_err(test_case.description);
        assert!(
            refusal.contains(test_case.expected_reason),
            "{}: {refusal}",
            test_case.description
        );
        assert_eq!(
            layout_sql(test_case.sql),
            test_case.sql,
            "{}",
            test_case.description
        );
    }
}
