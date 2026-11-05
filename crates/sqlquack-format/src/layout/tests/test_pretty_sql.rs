use sqlquack_syntax::Dialect;

use crate::layout::tests::test_types::PrettySqlTestCase;
use crate::pretty_sql;

#[test]
fn given_converted_sql_when_laid_out_then_it_takes_sqlcleans_layout() {
    let test_cases = [
        PrettySqlTestCase {
            description: "select items, CASE and GROUP BY and ORDER BY lists break onto lines",
            sql: r#"SELECT "ORDERS"."CUSTOMER_ID" AS "CUSTOMER_ID", CAST(sum("ORDERS"."TOTAL") AS DECIMAL(24,2)) AS "REVENUE", (CASE WHEN (CAST(count(*) AS DECIMAL(18,0)) > CAST(1 AS DECIMAL(1,0))) THEN CAST('repeat' AS VARCHAR) ELSE CAST('new' AS VARCHAR) END) AS "SEGMENT" FROM "ORDERS" AS "ORDERS" WHERE ("ORDERS"."STATUS" = 'shipped') GROUP BY "ORDERS"."CUSTOMER_ID" ORDER BY CAST(sum("ORDERS"."TOTAL") AS DECIMAL(24,2)) DESC NULLS FIRST"#,
            expected_sql: r#"SELECT
    "ORDERS"."CUSTOMER_ID" AS "CUSTOMER_ID",
    CAST(sum("ORDERS"."TOTAL") AS DECIMAL(24, 2)) AS "REVENUE",
    (CASE
        WHEN (CAST(count(*) AS DECIMAL(18, 0)) > CAST(1 AS DECIMAL(1, 0)))
        THEN CAST('repeat' AS VARCHAR)
        ELSE CAST('new' AS VARCHAR)
    END) AS "SEGMENT"
FROM "ORDERS" AS "ORDERS"
WHERE ("ORDERS"."STATUS" = 'shipped')
GROUP BY
    "ORDERS"."CUSTOMER_ID"
ORDER BY
    CAST(sum("ORDERS"."TOTAL") AS DECIMAL(24, 2)) DESC NULLS FIRST"#,
        },
        PrettySqlTestCase {
            description: "CTEs, joins and several conditions take their own lines",
            sql: r#"WITH "a" AS (SELECT "x"."id" AS "id", "x"."v" AS "v" FROM "x" AS "x" WHERE (("x"."v" > 1) AND ("x"."id" < 9))), "b" AS (SELECT 1 AS "one") SELECT "a"."id" AS "id" FROM "a" AS "a" LEFT JOIN "b" AS "b" ON ("a"."id" = "b"."one") INNER JOIN "c" ON "a"."id" = "c"."id" AND "c"."k" = 2 WHERE "a"."id" IN (SELECT "id" FROM "z") AND "a"."v" BETWEEN 2 AND 5 QUALIFY row_number() OVER (PARTITION BY "a"."id" ORDER BY "a"."v" DESC) = 1 LIMIT 10"#,
            expected_sql: r#"WITH "a" AS (
    SELECT
        "x"."id" AS "id",
        "x"."v" AS "v"
    FROM "x" AS "x"
    WHERE (("x"."v" > 1) AND ("x"."id" < 9))
),

"b" AS (
    SELECT 1 AS "one"
)

SELECT "a"."id" AS "id"
FROM "a" AS "a"
LEFT JOIN "b" AS "b"
    ON ("a"."id" = "b"."one")
INNER JOIN "c"
    ON "a"."id" = "c"."id"
    AND "c"."k" = 2
WHERE
    "a"."id" IN (
        SELECT "id"
        FROM "z"
    )
    AND "a"."v" BETWEEN 2 AND 5
QUALIFY row_number() OVER (PARTITION BY "a"."id" ORDER BY "a"."v" DESC) = 1
LIMIT 10"#,
        },
        PrettySqlTestCase {
            description: "a wide call wraps one argument per line, and set operations and statements split queries",
            sql: r#"SELECT coalesce("t"."aaaaaaaaaaaaaaaa", "t"."bbbbbbbbbbbbbbbbbbbb", "t"."cccccccccccccccccccccc", "t"."dddddddddddddddd") AS "x", 1 AS "y" FROM "t" AS "t" UNION ALL SELECT 2, 3 FROM "u"; SELECT 1"#,
            expected_sql: r#"SELECT
    coalesce(
        "t"."aaaaaaaaaaaaaaaa",
        "t"."bbbbbbbbbbbbbbbbbbbb",
        "t"."cccccccccccccccccccccc",
        "t"."dddddddddddddddd"
    ) AS "x",
    1 AS "y"
FROM "t" AS "t"
UNION ALL
SELECT
    2,
    3
FROM "u";

SELECT 1"#,
        },
        PrettySqlTestCase {
            description: "comments and the text inside strings are kept",
            sql: "SELECT 'a,  b' AS \"x\", -- note\n 2 AS \"y\" /* c */ FROM \"t\"",
            expected_sql: "SELECT\n    'a,  b' AS \"x\",\n    -- note\n    2 AS \"y\" /* c */\nFROM \"t\"",
        },
        PrettySqlTestCase {
            description: "bracketed set operations take their own lines, and division is spaced",
            sql: r#"SELECT 500/1000 AS "A", (SELECT 1 FROM "x") AS "B" FROM ((SELECT 1 AS "K" FROM "a") UNION ALL (SELECT 2 AS "K" FROM "b")) AS "T""#,
            expected_sql: r#"SELECT
    500 / 1000 AS "A",
    (
        SELECT 1
        FROM "x"
    ) AS "B"
FROM (
    (
        SELECT 1 AS "K"
        FROM "a"
    )
    UNION ALL
    (
        SELECT 2 AS "K"
        FROM "b"
    )
) AS "T""#,
        },
        PrettySqlTestCase {
            description: "a wide single select item goes below SELECT, its window one clause per line",
            sql: r#"SELECT LAG("INPUT"."AMOUNT", 2 IGNORE NULLS) OVER (PARTITION BY "INPUT"."CATEGORY" ORDER BY "INPUT"."SEQ" ASC NULLS LAST) AS "LAG_OFFSET_IGNORE_NULLS" FROM "INPUT" AS "INPUT""#,
            expected_sql: r#"SELECT
    LAG("INPUT"."AMOUNT", 2 IGNORE NULLS) OVER (
        PARTITION BY "INPUT"."CATEGORY"
        ORDER BY "INPUT"."SEQ" ASC NULLS LAST
    ) AS "LAG_OFFSET_IGNORE_NULLS"
FROM "INPUT" AS "INPUT""#,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            pretty_sql(test_case.sql, Dialect::DuckDB),
            test_case.expected_sql,
            "{}",
            test_case.description
        );
    }
}
