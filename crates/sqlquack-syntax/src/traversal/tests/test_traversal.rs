use crate::traversal::tests::helpers::labels;
use crate::traversal::tests::helpers::like_with_escape;
use crate::traversal::tests::helpers::statement;
use crate::traversal::tests::helpers::table_with_hint_and_identifier_function;
use crate::traversal::tests::helpers::walk_summary;
use crate::traversal::tests::test_types::DepthTestCase;
use crate::traversal::tests::test_types::OrderTestCase;
use crate::traversal::tests::test_types::SearchTestCase;
use crate::traversal::types::ExpressionWalk;

#[test]
fn given_expressions_when_walking_depth_first_then_nodes_come_in_pre_order() {
    let test_cases = [
        OrderTestCase {
            description: "a comparison visits itself, then its left and right sides",
            expression: statement("SELECT a = 1"),
            expected_labels: &["select:", "eq:", "column:a", "literal:1"],
        },
        OrderTestCase {
            description: "LIKE visits its ESCAPE expression last",
            expression: like_with_escape(),
            expected_labels: &["like:", "column:name", "literal:x%", "literal:!"],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            labels(test_case.expression.dfs()),
            test_case.expected_labels,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_expressions_when_walking_breadth_first_then_nodes_come_level_by_level() {
    let test_cases = [OrderTestCase {
        description: "both sides of the outer comparison come before the nested sum",
        expression: statement("SELECT a = b + 1"),
        expected_labels: &[
            "select:",
            "eq:",
            "column:a",
            "add:",
            "column:b",
            "literal:1",
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            labels(test_case.expression.bfs()),
            test_case.expected_labels,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_kind_when_searching_then_find_count_and_contains_agree() {
    let test_cases = [
        SearchTestCase {
            description: "the first column is found depth first",
            expression: statement("SELECT a = b + 1"),
            kind: "column",
            expected_first: Some("column:a"),
            expected_count: 2,
            expected_contains: true,
        },
        SearchTestCase {
            description: "literals are found",
            expression: statement("SELECT a = b + 1"),
            kind: "literal",
            expected_first: Some("literal:1"),
            expected_count: 1,
            expected_contains: true,
        },
        SearchTestCase {
            description: "a missing kind is not found",
            expression: statement("SELECT a = b + 1"),
            kind: "subquery",
            expected_first: None,
            expected_count: 0,
            expected_contains: false,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            walk_summary(&test_case.expression, test_case.kind),
            (
                test_case.expected_first.map(str::to_owned),
                test_case.expected_count,
                test_case.expected_contains
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_expressions_when_measuring_depth_then_leaves_are_zero_deep() {
    let test_cases = [
        DepthTestCase {
            description: "a literal is a leaf",
            expression: statement("SELECT 1").children()[0].clone(),
            expected_depth: 0,
        },
        DepthTestCase {
            description: "a comparison of leaves is one deep",
            expression: statement("SELECT a = 1").children()[0].clone(),
            expected_depth: 1,
        },
        DepthTestCase {
            description: "a nested sum adds a level",
            expression: statement("SELECT a = b + 1").children()[0].clone(),
            expected_depth: 2,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            test_case.expression.tree_depth(),
            test_case.expected_depth,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_expressions_when_listing_children_then_they_follow_field_order() {
    let test_cases = [
        OrderTestCase {
            description: "a literal has no children",
            expression: statement("SELECT 1").children()[0].clone(),
            expected_labels: &[],
        },
        OrderTestCase {
            description: "a comparison has its two sides",
            expression: statement("SELECT a = 1").children()[0].clone(),
            expected_labels: &["column:a", "literal:1"],
        },
        OrderTestCase {
            description: "a select lists projections, then its FROM table",
            expression: statement("SELECT a, b FROM t"),
            expected_labels: &["column:a", "column:b", "table:t"],
        },
        OrderTestCase {
            description: "a select includes its join sources",
            expression: statement("SELECT u.id FROM users u JOIN orders o ON u.id = o.user_id"),
            expected_labels: &["column:id", "table:users", "table:orders", "eq:"],
        },
        OrderTestCase {
            description: "a table includes its hints and identifier function",
            expression: table_with_hint_and_identifier_function(),
            expected_labels: &["column:table_hint", "identifier:dynamic_table"],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            labels(test_case.expression.children()),
            test_case.expected_labels,
            "{}",
            test_case.description
        );
    }
}
