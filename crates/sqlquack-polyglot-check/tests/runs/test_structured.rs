//! Reviewed structured entries cover only a polyglot command SQLQuack parses into the listed node.

use sqlquack_polyglot_check::expected::models::{ExpectedDifferences, StructuredStatement};

use crate::test_types::StructuredTestCase;

#[test]
fn given_a_command_polyglot_keeps_when_compared_then_only_the_listed_structure_is_matched() {
    let block = "BEGIN RETURN 1; END";
    let structured = r#"ok:[{"scripting_block":{"body":[]}}]"#;
    let test_cases = [
        StructuredTestCase {
            description: "a command SQLQuack parses into the listed node is matched",
            sql: block,
            polyglot: r#"ok:[{"command":{"this":"BEGIN RETURN 1; END"}}]"#,
            sqlquack: structured,
            expected_match: true,
        },
        StructuredTestCase {
            description: "raw SQL polyglot keeps, which SQLQuack parses into the listed node, is matched",
            sql: block,
            polyglot: r#"ok:[{"raw":{"sql":"BEGIN RETURN 1; END"}}]"#,
            sqlquack: structured,
            expected_match: true,
        },
        StructuredTestCase {
            description: "a structured polyglot tree is not matched",
            sql: block,
            polyglot: r#"ok:[{"select":{}}]"#,
            sqlquack: structured,
            expected_match: false,
        },
        StructuredTestCase {
            description: "a SQLQuack tree of another node is not matched",
            sql: block,
            polyglot: r#"ok:[{"command":{"this":"BEGIN RETURN 1; END"}}]"#,
            sqlquack: r#"ok:[{"command":{"this":"BEGIN RETURN 1; END"}}]"#,
            expected_match: false,
        },
        StructuredTestCase {
            description: "a SQLQuack error is not matched",
            sql: block,
            polyglot: r#"ok:[{"command":{"this":"BEGIN RETURN 1; END"}}]"#,
            sqlquack: "err:Parse",
            expected_match: false,
        },
        StructuredTestCase {
            description: "a statement without the marker is not matched",
            sql: "DECLARE x INT;",
            polyglot: r#"ok:[{"command":{"this":"DECLARE x INT;"}}]"#,
            sqlquack: structured,
            expected_match: false,
        },
    ];
    let expected = ExpectedDifferences {
        structured: vec![StructuredStatement {
            id: "snowflake_scripting_block".to_owned(),
            marker: "BEGIN".to_owned(),
            sqlquack_node: "scripting_block".to_owned(),
            note: "a test entry".to_owned(),
        }],
        ..ExpectedDifferences::default()
    };

    for test_case in test_cases {
        assert_eq!(
            expected
                .structured_for(test_case.sql, test_case.polyglot, test_case.sqlquack)
                .is_some(),
            test_case.expected_match,
            "{}",
            test_case.description
        );
    }
}
