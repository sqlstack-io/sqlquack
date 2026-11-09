use crate::snowflake::classification::tests::helpers::{
    line_and_column, missing_keys, unclassified_nodes, unlocated_constructs,
};
use crate::snowflake::classification::tests::test_types::{
    CompletenessTestCase, ConstructTestCase, LocationTestCase,
};

#[test]
fn given_snowflake_query_when_classifying_then_reports_each_construct_family() {
    let test_cases = [
        ConstructTestCase {
            description: "generic functions are keyed by upper-case name",
            sql: "SELECT zeroifnull(o.amount) AS amount FROM orders o",
            expected_keys: &["fn:ZEROIFNULL", "node:alias", "node:column", "node:select"],
        },
        ConstructTestCase {
            description: "casts record the node, the base type and the `::` spelling",
            sql: "SELECT o.amount::NUMBER(10, 2) AS amount FROM orders o",
            expected_keys: &["node:cast", "cast:DECIMAL", "syntax:double_colon_cast"],
        },
        ConstructTestCase {
            description: "function-style casts record the node and the base type",
            sql: "SELECT CAST(o.ordered_at AS DATE) FROM orders o",
            expected_keys: &["node:cast", "cast:DATE"],
        },
        ConstructTestCase {
            description: "join kinds and query clauses are constructs",
            sql: "SELECT c.customer_id FROM customers c LEFT JOIN orders o \
                  ON o.customer_id = c.customer_id WHERE o.order_id IS NULL \
                  QUALIFY ROW_NUMBER() OVER (PARTITION BY c.customer_id ORDER BY c.created_at) = 1",
            expected_keys: &[
                "join:left",
                "clause:where",
                "clause:qualify",
                "node:eq",
                "node:is_null",
                "fn:ROW_NUMBER",
                "node:window_function",
            ],
        },
        ConstructTestCase {
            description: "set operations record their ALL form",
            sql: "SELECT product_id FROM products UNION ALL SELECT product_id FROM inventory",
            expected_keys: &["node:union", "clause:union_all"],
        },
        ConstructTestCase {
            description: "GROUP BY ALL and ORDER BY ALL have keys of their own beside the clause",
            sql: "SELECT region, COUNT(*) FROM orders GROUP BY ALL ORDER BY ALL",
            expected_keys: &[
                "clause:group_by",
                "clause:group_by_all",
                "clause:order_by",
                "clause:order_by_all",
            ],
        },
        ConstructTestCase {
            description: "WITH RECURSIVE has a key of its own beside the clause",
            sql: "WITH RECURSIVE r AS (SELECT 1 AS n) SELECT n FROM r",
            expected_keys: &["clause:with", "clause:with_recursive"],
        },
        ConstructTestCase {
            description: "a SAMPLE on a table reference is a sample clause",
            sql: "SELECT order_id FROM orders SAMPLE (10)",
            expected_keys: &["clause:sample"],
        },
        ConstructTestCase {
            description: "project functions collapse into one construct",
            sql: "SELECT __sqlquack_udf(o.status) FROM orders o",
            expected_keys: &["fn:<udf>"],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            missing_keys(test_case.sql, test_case.expected_keys),
            Vec::<String>::new(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_any_query_when_classifying_then_every_node_has_a_located_construct() {
    let test_cases = [
        CompletenessTestCase {
            description: "a CTE with aggregates, joins, conditionals and ordering",
            sql: "WITH recent AS (SELECT customer_id, SUM(amount) AS total FROM orders \
                  WHERE ordered_at >= DATEADD('day', -30, CURRENT_DATE()) GROUP BY customer_id) \
                  SELECT c.name, COALESCE(r.total, 0) AS total, IFF(r.total > 100, 'high', 'low') \
                  FROM customers c LEFT JOIN recent r ON r.customer_id = c.customer_id \
                  ORDER BY total DESC NULLS LAST LIMIT 10",
            expected_unclassified_nodes: 0,
            expected_unlocated_constructs: 0,
        },
        CompletenessTestCase {
            description: "semi-structured paths and a lateral flatten",
            sql: "SELECT o.payload:customer.name::VARCHAR AS name, f.value \
                  FROM orders o, LATERAL FLATTEN(input => o.payload:items) f",
            expected_unclassified_nodes: 0,
            expected_unlocated_constructs: 0,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            unclassified_nodes(test_case.sql),
            test_case.expected_unclassified_nodes,
            "{}",
            test_case.description
        );
        assert_eq!(
            unlocated_constructs(test_case.sql),
            test_case.expected_unlocated_constructs,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_multi_line_query_when_classifying_then_locations_point_into_the_source() {
    let test_cases = [
        LocationTestCase {
            description: "a column keeps its own line and column",
            sql: "SELECT\n  o.order_id\nFROM orders o",
            key: "node:column",
            expected_line_and_column: Some([2, 3]),
        },
        LocationTestCase {
            description: "a call is placed at its name",
            sql: "SELECT\n  o.order_id,\n  ZEROIFNULL(o.amount) AS amount\nFROM orders o",
            key: "fn:ZEROIFNULL",
            expected_line_and_column: Some([3, 3]),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            line_and_column(test_case.sql, test_case.key),
            test_case.expected_line_and_column,
            "{}",
            test_case.description
        );
    }
}
