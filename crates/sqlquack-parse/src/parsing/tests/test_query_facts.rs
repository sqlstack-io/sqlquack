//! Facts a located tree gives about queries: FROM items after commas, and set operations.

use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};

use crate::parsing::main::parse_source::parse_source;
use crate::parsing::tests::test_types::{FromItemTestCase, SetOperationTestCase};

#[test]
fn given_from_items_after_commas_when_classified_then_the_lateral_ones_are_marked() {
    let test_cases = [
        FromItemTestCase {
            description: "a plain comma join",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, c",
            expected_after_commas: &[(15, false)],
        },
        FromItemTestCase {
            description: "TABLE(FLATTEN)",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, TABLE(FLATTEN(input => o.items)) f",
            expected_after_commas: &[(15, true)],
        },
        FromItemTestCase {
            description: "a table function over a window",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM p, TABLE(smooth(p.amount) OVER (PARTITION BY p.c)) s",
            expected_after_commas: &[(15, true)],
        },
        FromItemTestCase {
            description: "TABLE of a name is a relation",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, TABLE('customers') c",
            expected_after_commas: &[(15, false)],
        },
        FromItemTestCase {
            description: "LATERAL FLATTEN",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, LATERAL FLATTEN(input => o.items) f",
            expected_after_commas: &[(15, true)],
        },
        FromItemTestCase {
            description: "a lateral query",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, LATERAL (SELECT o.id) f",
            expected_after_commas: &[(15, true)],
        },
        FromItemTestCase {
            description: "a derived table",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, (SELECT id FROM c) d",
            expected_after_commas: &[(15, false)],
        },
        FromItemTestCase {
            description: "inline values",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM o, (VALUES (1), (2)) v(id)",
            expected_after_commas: &[(15, false)],
        },
        FromItemTestCase {
            description: "commas after a join",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 FROM a JOIN b ON a.x = b.x, TABLE(FLATTEN(a.y)) f, c",
            expected_after_commas: &[(35, true), (58, false)],
        },
        FromItemTestCase {
            description: "a comma between named windows is no join",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT SUM(a) OVER w1 FROM t WINDOW w1 AS (ORDER BY a), w2 AS (ORDER BY b)",
            expected_after_commas: &[],
        },
        FromItemTestCase {
            description: "correlated UNNEST",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT 1 FROM t, UNNEST(t.arr) u(x)",
            expected_after_commas: &[(15, true)],
        },
        FromItemTestCase {
            description: "UNNEST of a literal reads nothing before it",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT 1 FROM t, UNNEST([1, 2]) u(x)",
            expected_after_commas: &[(15, false)],
        },
        #[cfg(feature = "postgres")]
        FromItemTestCase {
            description: "a PostgreSQL function",
            parse: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM a, jsonb_array_elements(a.j) e",
            expected_after_commas: &[(15, true)],
        },
        #[cfg(feature = "postgres")]
        FromItemTestCase {
            description: "UNNEST WITH ORDINALITY",
            parse: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM t, UNNEST(t.arr) WITH ORDINALITY u(x, n)",
            expected_after_commas: &[(15, true)],
        },
        #[cfg(feature = "postgres")]
        FromItemTestCase {
            description: "PostgreSQL keeps joins inside the item",
            parse: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM a JOIN b ON a.x = b.x, c",
            expected_after_commas: &[(35, false)],
        },
        #[cfg(feature = "bigquery")]
        FromItemTestCase {
            description: "UNNEST WITH OFFSET",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM t, UNNEST(t.arr) AS x WITH OFFSET",
            expected_after_commas: &[(15, true)],
        },
        #[cfg(feature = "bigquery")]
        FromItemTestCase {
            description: "a correlated array path",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM t, t.arr",
            expected_after_commas: &[(15, true)],
        },
        #[cfg(feature = "bigquery")]
        FromItemTestCase {
            description: "an array path through an alias",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM orders AS o, o.lines",
            expected_after_commas: &[(25, true)],
        },
        #[cfg(feature = "bigquery")]
        FromItemTestCase {
            description: "a qualified table is no array path",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM t, sales.t2",
            expected_after_commas: &[(15, false)],
        },
        #[cfg(feature = "databricks")]
        FromItemTestCase {
            description: "LATERAL explode",
            parse: |sql| {
                sqlquack_syntax_databricks::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT 1 FROM t, LATERAL explode(t.a) e",
            expected_after_commas: &[(15, true)],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let mut items: Vec<(usize, bool)> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(Expression::as_select)
            .flat_map(|select| select.items_after_commas())
            .filter_map(|item| {
                item.comma
                    .0
                    .map(|comma| (comma.start, item.may_read_earlier()))
            })
            .collect();
        items.sort_unstable();
        assert!(!statements.is_empty(), "{}: parses", test_case.description);
        assert_eq!(
            items, test_case.expected_after_commas,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_set_operations_when_walked_then_each_gives_by_name_and_its_sides_column_counts() {
    let test_cases = [
        SetOperationTestCase {
            description: "BY NAME",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 AS a, 'x' AS b UNION ALL BY NAME SELECT 'y' AS b",
            expected_operations: &[(true, Some(2), Some(1))],
        },
        SetOperationTestCase {
            description: "ORDER BY commas are no columns",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 'a' AS k, 1 AS v UNION ALL SELECT 'b', 2 ORDER BY k, v",
            expected_operations: &[(false, Some(2), Some(2))],
        },
        SetOperationTestCase {
            description: "multiplication is no wildcard",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT a * 2, b FROM t UNION ALL SELECT c FROM u",
            expected_operations: &[(false, Some(2), Some(1))],
        },
        SetOperationTestCase {
            description: "parenthesised branches",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "(SELECT a, b FROM x) UNION ALL (SELECT c FROM y)",
            expected_operations: &[(false, Some(2), Some(1))],
        },
        SetOperationTestCase {
            description: "a wildcard has no count",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT t.* FROM t UNION ALL SELECT c FROM u",
            expected_operations: &[(false, None, Some(1))],
        },
        SetOperationTestCase {
            description: "COLUMNS has no count",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT COLUMNS('a.*') FROM t UNION ALL SELECT c FROM u",
            expected_operations: &[(false, None, Some(1))],
        },
        SetOperationTestCase {
            description: "VALUES counts its first row",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT a, b FROM t UNION ALL VALUES (1, 2)",
            expected_operations: &[(false, Some(2), Some(2))],
        },
        SetOperationTestCase {
            description: "a chain counts each side",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1, 2 UNION ALL SELECT 3, 4 UNION ALL SELECT 5",
            expected_operations: &[(false, Some(2), Some(1)), (false, Some(2), Some(2))],
        },
        SetOperationTestCase {
            description: "a chain of names has no count",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT 1 AS a UNION BY NAME SELECT 2 AS b UNION ALL SELECT 3, 4",
            expected_operations: &[(false, None, Some(2)), (true, Some(1), Some(1))],
        },
        #[cfg(feature = "bigquery")]
        SetOperationTestCase {
            description: "a star EXCEPT is no set operation",
            parse: |sql| {
                sqlquack_syntax_bigquery::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT * EXCEPT (secret) FROM users",
            expected_operations: &[],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let operations: Vec<(bool, Option<usize>, Option<usize>)> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(Expression::set_operation)
            .map(|set| {
                (
                    set.by_name,
                    set.left.column_count(),
                    set.right.column_count(),
                )
            })
            .collect();
        assert!(!statements.is_empty(), "{}: parses", test_case.description);
        assert_eq!(
            operations, test_case.expected_operations,
            "{}",
            test_case.description
        );
    }
}
