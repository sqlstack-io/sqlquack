//! Reviewed kept nodes cover only wrappers polyglot drops, in statements with the marker.

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_polyglot_check::expected::models::{ExpectedDifferences, KeptNode};
use sqlquack_syntax::Dialect;

use crate::helpers::summary_of;
use crate::test_types::{KeptNodeTestCase, QuotingTreesTestCase};

#[test]
fn given_unary_plus_when_compared_then_only_the_kept_operator_is_explained() {
    let test_cases = [
        KeptNodeTestCase {
            description: "a unary plus on a literal is a kept node",
            sql: "SELECT + 1",
            expected_kept_nodes: 1,
            expected_differences: 0,
        },
        KeptNodeTestCase {
            description: "a unary plus on a column is a kept node",
            sql: "SELECT +IF FROM t",
            expected_kept_nodes: 1,
            expected_differences: 0,
        },
        KeptNodeTestCase {
            description: "a unary minus gives identical trees",
            sql: "SELECT - 1",
            expected_kept_nodes: 0,
            expected_differences: 0,
        },
        KeptNodeTestCase {
            description: "a binary plus gives identical trees",
            sql: "SELECT 1 + 2",
            expected_kept_nodes: 0,
            expected_differences: 0,
        },
    ];

    for test_case in test_cases {
        let summary = summary_of(&[CorpusEntry {
            dialect: Dialect::Snowflake,
            sql: test_case.sql.to_owned(),
            from: test_case.description.to_owned(),
        }]);
        assert_eq!(
            (
                summary.kept_nodes.values().sum::<usize>(),
                summary.differences.len()
            ),
            (
                test_case.expected_kept_nodes,
                test_case.expected_differences
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_trees_with_a_marker_when_matched_then_only_dropped_wrappers_are_allowed() {
    let column = |name: &str| format!(r#"{{"column":{{"name":"{name}"}}}}"#);
    let pos = |inner: String| format!(r#"{{"pos":{{"this":{inner}}}}}"#);
    let tree = |first: String, second: String| format!(r#"ok:[{{"select":[{first},{second}]}}]"#);
    let test_cases = [
        QuotingTreesTestCase {
            description: "a wrapper polyglot drops is the only difference",
            sql: "SELECT +a, b",
            polyglot: tree(column("a"), column("b")),
            sqlquack: tree(pos(column("a")), column("b")),
            expected_match: true,
        },
        QuotingTreesTestCase {
            description: "nested wrappers polyglot drops are the only difference",
            sql: "SELECT + +a, b",
            polyglot: tree(column("a"), column("b")),
            sqlquack: tree(pos(pos(column("a"))), column("b")),
            expected_match: true,
        },
        QuotingTreesTestCase {
            description: "a different name beside the wrapper is not matched",
            sql: "SELECT +a, b",
            polyglot: tree(column("a"), column("c")),
            sqlquack: tree(pos(column("a")), column("b")),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a wrapper polyglot keeps and SQLQuack drops is not matched",
            sql: "SELECT +a, b",
            polyglot: tree(pos(column("a")), column("b")),
            sqlquack: tree(column("a"), column("b")),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a wrapper with a field besides its operand is not matched",
            sql: "SELECT +a, b",
            polyglot: tree(column("a"), column("b")),
            sqlquack: tree(
                format!(r#"{{"pos":{{"this":{},"span":null}}}}"#, column("a")),
                column("b"),
            ),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a statement without the marker is not matched",
            sql: "SELECT a, b",
            polyglot: tree(column("a"), column("b")),
            sqlquack: tree(pos(column("a")), column("b")),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a polyglot error is not matched",
            sql: "SELECT +a, b",
            polyglot: "err:Parse".to_owned(),
            sqlquack: tree(pos(column("a")), column("b")),
            expected_match: false,
        },
    ];
    let expected = ExpectedDifferences {
        kept_nodes: vec![KeptNode {
            id: "unary_plus".to_owned(),
            marker: "+".to_owned(),
            node: "pos".to_owned(),
            note: "a test entry".to_owned(),
        }],
        ..ExpectedDifferences::default()
    };

    for test_case in test_cases {
        assert_eq!(
            expected
                .kept_node_for(test_case.sql, &test_case.polyglot, &test_case.sqlquack)
                .is_some(),
            test_case.expected_match,
            "{}",
            test_case.description
        );
    }
}
