//! Reviewed kept quoting covers only names polyglot leaves unquoted, in statements with the marker.

use sqlquack_polyglot_check::corpora::models::CorpusEntry;
use sqlquack_polyglot_check::expected::models::{ExpectedDifferences, KeptQuoting};
use sqlquack_syntax::Dialect;

use crate::helpers::summary_of;
use crate::test_types::{KeptQuotingTestCase, QuotingTreesTestCase};

#[test]
fn given_quoted_exclude_names_when_compared_then_only_the_marked_statement_is_kept_quoting() {
    let test_cases = [
        KeptQuotingTestCase {
            description: "a quoted name in a star's EXCLUDE list is kept quoting",
            sql: "SELECT * EXCLUDE (\"note\") FROM orders",
            expected_kept_quoting: 1,
            expected_differences: 0,
        },
        KeptQuotingTestCase {
            description: "a quoted EXCLUDE name inside a call is kept quoting",
            sql: "SELECT OBJECT_CONSTRUCT(* EXCLUDE (\"note\")) AS o FROM orders",
            expected_kept_quoting: 1,
            expected_differences: 0,
        },
        KeptQuotingTestCase {
            description: "an unquoted EXCLUDE name gives identical trees",
            sql: "SELECT * EXCLUDE (note) FROM orders",
            expected_kept_quoting: 0,
            expected_differences: 0,
        },
        KeptQuotingTestCase {
            description: "a quoted EXCLUDE name without parentheses lacks the marker and still differs",
            sql: "SELECT * EXCLUDE \"note\" FROM orders",
            expected_kept_quoting: 0,
            expected_differences: 1,
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
                summary.kept_quoting.values().sum::<usize>(),
                summary.differences.len()
            ),
            (
                test_case.expected_kept_quoting,
                test_case.expected_differences
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_trees_with_a_marker_when_matched_then_only_dropped_quoting_is_allowed() {
    let identifier = |name: &str, quoted: bool| {
        format!(r#"{{"name":"{name}","quoted":{quoted},"trailing_comments":[]}}"#)
    };
    let tree = |first: String, second: String| format!(r#"ok:[{{"except":[{first},{second}]}}]"#);
    let quoted_only = tree(identifier("note", true), identifier("b", false));
    let test_cases = [
        QuotingTreesTestCase {
            description: "polyglot unquoted and SQLQuack quoted is the only difference",
            sql: "SELECT * EXCLUDE (\"note\", b) FROM orders",
            polyglot: tree(identifier("note", false), identifier("b", false)),
            sqlquack: quoted_only.clone(),
            expected_match: true,
        },
        QuotingTreesTestCase {
            description: "a different name beside the dropped quoting is not matched",
            sql: "SELECT * EXCLUDE (\"note\", b) FROM orders",
            polyglot: tree(identifier("note", false), identifier("c", false)),
            sqlquack: quoted_only.clone(),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "quoting SQLQuack drops and polyglot keeps is not matched",
            sql: "SELECT * EXCLUDE (\"note\", b) FROM orders",
            polyglot: tree(identifier("note", true), identifier("b", true)),
            sqlquack: quoted_only.clone(),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a quoted flag on a node that is not a name is not matched",
            sql: "SELECT * EXCLUDE (\"note\") FROM orders",
            polyglot: r#"ok:[{"star":{"quoted":false}}]"#.to_owned(),
            sqlquack: r#"ok:[{"star":{"quoted":true}}]"#.to_owned(),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a field only one tree has is not matched",
            sql: "SELECT * EXCLUDE (\"note\") FROM orders",
            polyglot: r#"ok:[{"name":"note","quoted":false}]"#.to_owned(),
            sqlquack: r#"ok:[{"name":"note","quoted":true,"span":null}]"#.to_owned(),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a statement without the marker is not matched",
            sql: "SELECT * EXCLUDE \"note\" FROM orders",
            polyglot: tree(identifier("note", false), identifier("b", false)),
            sqlquack: quoted_only.clone(),
            expected_match: false,
        },
        QuotingTreesTestCase {
            description: "a polyglot error is not matched",
            sql: "SELECT * EXCLUDE (\"note\") FROM orders",
            polyglot: "err:Parse".to_owned(),
            sqlquack: quoted_only,
            expected_match: false,
        },
    ];
    let expected = ExpectedDifferences {
        kept_quotings: vec![KeptQuoting {
            id: "star_exclude_quoted_name".to_owned(),
            marker: "EXCLUDE(\"".to_owned(),
            note: "a test entry".to_owned(),
        }],
        ..ExpectedDifferences::default()
    };

    for test_case in test_cases {
        assert_eq!(
            expected
                .kept_quoting_for(test_case.sql, &test_case.polyglot, &test_case.sqlquack)
                .is_some(),
            test_case.expected_match,
            "{}",
            test_case.description
        );
    }
}
