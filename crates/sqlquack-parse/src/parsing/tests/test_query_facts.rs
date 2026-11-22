//! Facts a located tree gives about queries: FROM items after commas, and set operations.

use sqlquack_syntax::traversal::models::QueryScopes;
use sqlquack_syntax::{Dialect, Expression, ExpressionWalk};

use crate::parsing::main::parse_source::parse_source;
use crate::parsing::tests::test_types::{
    BranchTestCase, FromItemTestCase, JoinedItemTestCase, QueryScopeTestCase, SetOperationTestCase,
};

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

#[test]
fn given_qualified_columns_when_scoped_then_each_sees_the_relations_of_its_query_and_those_around()
{
    let test_cases = [
        QueryScopeTestCase {
            description: "aliases and a join",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT o.id, c.name, x.zip FROM orders o JOIN customers c ON o.customer_id = c.cid",
            expected_visible: &[
                ("cid", true),
                ("customer_id", true),
                ("id", true),
                ("name", true),
                ("zip", false),
            ],
        },
        QueryScopeTestCase {
            description: "an alias hides the table name",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT orders.id FROM orders o",
            expected_visible: &[("id", false)],
        },
        QueryScopeTestCase {
            description: "a subquery sees the queries around it",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT (SELECT MAX(p.paid) FROM payments p WHERE p.order_id = o.id) FROM orders o",
            expected_visible: &[("id", true), ("order_id", true), ("paid", true)],
        },
        QueryScopeTestCase {
            description: "a derived table does not see its neighbours",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT s.a FROM orders o, (SELECT o.id AS a FROM items i) s",
            expected_visible: &[("a", true), ("id", false)],
        },
        QueryScopeTestCase {
            description: "a lateral query does",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT s.a FROM orders o, LATERAL (SELECT o.id AS a) s",
            expected_visible: &[("a", true), ("id", true)],
        },
        QueryScopeTestCase {
            description: "a CTE does not see the query that reads it",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "WITH r AS (SELECT o.id FROM items i) SELECT o.total FROM orders o",
            expected_visible: &[("id", false), ("total", true)],
        },
        QueryScopeTestCase {
            description: "a derived table in a subquery sees past it",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT o.total FROM orders o WHERE EXISTS (SELECT 1 FROM (SELECT o.id FROM items i) s)",
            expected_visible: &[("id", true), ("total", true)],
        },
        QueryScopeTestCase {
            description: "set operation branches see their own relations",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT o.id FROM orders o UNION ALL SELECT o.rid FROM returns r",
            expected_visible: &[("id", true), ("rid", false)],
        },
        QueryScopeTestCase {
            description: "a table function reads an earlier item",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT f.value FROM orders o, LATERAL FLATTEN(input => o.items) f",
            expected_visible: &[("items", true), ("value", true)],
        },
        QueryScopeTestCase {
            description: "a qualified call is no qualifier",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT util.parse_day(o.day) FROM orders o",
            expected_visible: &[("day", true)],
        },
        #[cfg(feature = "postgres")]
        QueryScopeTestCase {
            description: "PostgreSQL nested joins",
            parse: |sql| {
                sqlquack_syntax_postgres::parse_strict(sql)
                    .ok()
                    .map(|parsed| parsed.statements)
            },
            sql: "SELECT b.y FROM a JOIN b ON a.x = b.y, c WHERE c.z = 1",
            expected_visible: &[("x", true), ("y", true), ("y", true), ("z", true)],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let scoped: Vec<QueryScopes<'_>> =
            statements.iter().map(Expression::query_scopes).collect();
        let mut visible: Vec<(&str, bool)> = scoped
            .iter()
            .flat_map(|query| query.qualified_columns())
            .map(|read| {
                let seen: bool = read
                    .visible
                    .iter()
                    .any(|relation| relation.name.eq_ignore_ascii_case(read.qualifier));
                (read.column.name.name.as_str(), seen)
            })
            .collect();
        visible.sort_unstable();
        assert_eq!(
            visible, test_case.expected_visible,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_set_operation_when_reading_the_branches_at_its_keyword_then_they_are_the_adjacent_ones()
{
    let test_cases = [
        BranchTestCase {
            description: "a chain's last branch meets the next",
            parse: |sql| parse_source(sql, Dialect::Snowflake).ok(),
            sql: "SELECT 1 UNION ALL SELECT * FROM t UNION ALL SELECT 3, 4",
            expected_sides: [(true, None), (false, Some(2))],
        },
        BranchTestCase {
            description: "parentheses are opened",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT 1, 2 UNION ALL (SELECT t.* FROM t UNION ALL SELECT 3)",
            expected_sides: [(false, Some(2)), (true, None)],
        },
        BranchTestCase {
            description: "multiplication and VALUES are no wildcard",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT qty * price FROM t UNION ALL VALUES (1)",
            expected_sides: [(false, Some(1)), (false, Some(1))],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let sides: Vec<(bool, Option<usize>)> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .filter_map(Expression::set_operation)
            .take(1)
            .flat_map(|set| [set.left.last_branch(), set.right.first_branch()])
            .map(|branch| (branch.projects_wildcard(), branch.column_count()))
            .collect();
        assert_eq!(sides, test_case.expected_sides, "{}", test_case.description);
    }
}

#[test]
fn given_joins_when_their_item_is_lateral_then_it_may_read_the_items_before_it() {
    let test_cases = [
        JoinedItemTestCase {
            description: "a lateral query, and a plain relation",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT 1 FROM o LEFT JOIN LATERAL (SELECT o.id) AS l ON TRUE JOIN c ON TRUE",
            expected_lateral: &[true, false],
        },
        JoinedItemTestCase {
            description: "correlated UNNEST",
            parse: |sql| parse_source(sql, Dialect::DuckDB).ok(),
            sql: "SELECT 1 FROM o LEFT JOIN UNNEST(o.items) AS u(x) ON TRUE",
            expected_lateral: &[true],
        },
    ];

    for test_case in test_cases {
        let statements: Vec<Expression> = (test_case.parse)(test_case.sql).unwrap_or_default();
        let lateral: Vec<bool> = statements
            .iter()
            .flat_map(|statement| statement.dfs())
            .flat_map(Expression::own_joins)
            .map(|join| join.may_read_earlier())
            .collect();
        assert_eq!(
            lateral, test_case.expected_lateral,
            "{}",
            test_case.description
        );
    }
}
