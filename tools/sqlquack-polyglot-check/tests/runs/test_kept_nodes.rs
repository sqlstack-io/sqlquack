//! Reviewed kept nodes and fields cover only what polyglot drops or infers, in statements with the marker.

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_polyglot_check::expected::models::{
    ExpectedDifferences, KeptAll, KeptField, KeptNode,
};
use sqlquack_syntax::Dialect;

use crate::helpers::summary_of;
use crate::test_types::{KeptAllTestCase, KeptNodeTestCase, QuotingTreesTestCase};

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

#[test]
fn given_trees_with_a_marker_when_matched_then_only_the_kept_field_may_differ() {
    let tree = |scope: &str, name: &str| {
        format!(r#"ok:[{{"show":{{"this":"TABLES","scope_kind":{scope},"scope":"{name}"}}}}]"#)
    };
    let test_cases = [
        QuotingTreesTestCase {
            description: "a field polyglot infers is the only difference",
            sql: "SHOW TABLES IN a.b",
            polyglot: tree(r#""SCHEMA""#, "a.b"),
            sqlquack: tree("null", "a.b"),
            expected_match: true,
        },
        QuotingTreesTestCase {
            description: "a field polyglot lacks is the only difference",
            sql: "SHOW TABLES IN a.b",
            polyglot: r#"ok:[{"show":{"this":"TABLES","scope":"a.b"}}]"#.to_owned(),
            sqlquack: tree("null", "a.b"),
            expected_match: true,
        },
        QuotingTreesTestCase {
            description: "another difference beside the field is not matched",
            sql: "SHOW TABLES IN a.b",
            polyglot: tree(r#""SCHEMA""#, "a.b"),
            sqlquack: tree("null", "a.c"),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a statement without the marker is not matched",
            sql: "DESC TABLE a.b",
            polyglot: tree(r#""SCHEMA""#, "a.b"),
            sqlquack: tree("null", "a.b"),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a polyglot error is not matched",
            sql: "SHOW TABLES IN a.b",
            polyglot: "err:Parse".to_owned(),
            sqlquack: tree("null", "a.b"),
            expected_match: false,
        },
    ];
    let expected = ExpectedDifferences {
        kept_fields: vec![KeptField {
            id: "show_container".to_owned(),
            marker: "SHOW".to_owned(),
            field: "scope_kind".to_owned(),
            note: "a test entry".to_owned(),
        }],
        ..ExpectedDifferences::default()
    };

    for test_case in test_cases {
        assert_eq!(
            expected
                .kept_field_for(test_case.sql, &test_case.polyglot, &test_case.sqlquack)
                .is_some(),
            test_case.expected_match,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_order_by_all_when_compared_then_the_kept_all_is_explained_with_its_options() {
    let test_cases = [
        KeptAllTestCase {
            description: "ORDER BY ALL is a kept ALL",
            sql: "SELECT a, b FROM t ORDER BY ALL",
            expected_kept_fields: 1,
            expected_differences: 0,
        },
        KeptAllTestCase {
            description: "ORDER BY ALL DESC NULLS FIRST carries its options",
            sql: "SELECT a, b FROM t ORDER BY ALL DESC NULLS FIRST",
            expected_kept_fields: 1,
            expected_differences: 0,
        },
        KeptAllTestCase {
            description: "ORDER BY a column gives identical trees",
            sql: "SELECT a, b FROM t ORDER BY a DESC",
            expected_kept_fields: 0,
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
                summary.kept_fields.values().sum::<usize>(),
                summary.differences.len()
            ),
            (
                test_case.expected_kept_fields,
                test_case.expected_differences
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_trees_with_a_marker_when_matched_then_only_the_folded_all_may_differ() {
    let key = |this: &str, desc: bool| {
        format!(r#"{{"this":{this},"desc":{desc},"nulls_first":null,"explicit_asc":false}}"#)
    };
    let all_key = |desc: bool| key(r#"{"var":{"this":"ALL"}}"#, desc);
    let polyglot =
        |keys: String| format!(r#"ok:[{{"select":{{"order_by":{{"expressions":[{keys}]}}}}}}]"#);
    let sqlquack = |desc: bool| {
        format!(
            r#"ok:[{{"select":{{"order_by":{{"expressions":[],"all":{{"desc":{desc},"nulls_first":null,"explicit_asc":false}}}}}}}}]"#
        )
    };
    let test_cases = [
        QuotingTreesTestCase {
            description: "ALL read as a key is the only difference",
            sql: "SELECT a FROM t ORDER BY ALL DESC",
            polyglot: polyglot(all_key(true)),
            sqlquack: sqlquack(true),
            expected_match: true,
        },
        QuotingTreesTestCase {
            description: "a different direction is not matched",
            sql: "SELECT a FROM t ORDER BY ALL DESC",
            polyglot: polyglot(all_key(false)),
            sqlquack: sqlquack(true),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a key other than ALL is not matched",
            sql: "SELECT a FROM t ORDER BY ALL",
            polyglot: polyglot(key(r#"{"column":{"name":"a"}}"#, false)),
            sqlquack: sqlquack(false),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "ALL beside another key is not matched",
            sql: "SELECT a FROM t ORDER BY ALL",
            polyglot: polyglot(format!(
                "{},{}",
                all_key(false),
                key(r#"{"column":{"name":"a"}}"#, false)
            )),
            sqlquack: sqlquack(false),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a statement without the marker is not matched",
            sql: "SELECT a FROM t ORDER BY a",
            polyglot: polyglot(all_key(false)),
            sqlquack: sqlquack(false),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a polyglot error is not matched",
            sql: "SELECT a FROM t ORDER BY ALL",
            polyglot: "err:Parse".to_owned(),
            sqlquack: sqlquack(false),
            expected_match: false,
        },
    ];
    let expected = ExpectedDifferences {
        kept_alls: vec![KeptAll {
            id: "order_by_all".to_owned(),
            marker: "ORDERBYALL".to_owned(),
            clause: "order_by".to_owned(),
            note: "a test entry".to_owned(),
        }],
        ..ExpectedDifferences::default()
    };

    for test_case in test_cases {
        assert_eq!(
            expected
                .kept_all_for(test_case.sql, &test_case.polyglot, &test_case.sqlquack)
                .is_some(),
            test_case.expected_match,
            "{}",
            test_case.description
        );
    }
}
