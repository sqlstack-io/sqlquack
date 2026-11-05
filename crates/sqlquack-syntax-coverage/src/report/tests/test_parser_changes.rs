use crate::check::tests::helpers::{
    append_receipt, opaque, receipt_line, rejecting_select_17, select_17, structural,
    verified_tree, with_fields,
};
use crate::formats::constants::MARK_DOCUMENTED;
use crate::report::main::diff_snapshots::diff_snapshots;
use crate::report::models::SnapshotDiff;
use crate::report::tests::helpers::{
    ids, mark_select_from, observed_select_17, observed_select_from, snapshot_with,
};
use crate::report::tests::test_types::ParserChangeTestCase;

/// The shapes of the four regressions #55 brought on observe rows (audit 38bed222), and controls.
#[test]
fn given_receipted_cases_of_any_expectation_when_the_parser_changes_then_the_diff_flags_contradictions()
 {
    let test_cases = [
        ParserChangeTestCase {
            description: "an observe case Snowflake accepted, now rejected (create-database-role)",
            mutate: |root| observed_select_17(root, "accepted", 0),
            base: structural,
            head: rejecting_select_17,
            expected_parsed_cases_refused: vec!["select-17"],
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "an observe case that failed semantic analysis, now rejected (database-role-clone)",
            mutate: |root| observed_select_17(root, "semantic_error", 2003),
            base: structural,
            head: rejecting_select_17,
            expected_parsed_cases_refused: vec!["select-17"],
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "an observe case Snowflake parsed, opaque before and rejected now (create-stream-clone)",
            mutate: |root| observed_select_17(root, "semantic_error", 1422),
            base: opaque,
            head: rejecting_select_17,
            expected_parsed_cases_refused: vec!["select-17"],
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "an observe case refused with a positioned 1003, now accepted (create-database)",
            mutate: |root| observed_select_from(root, 1003),
            base: structural,
            head: opaque,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: vec!["select-from"],
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "an observe case refused with 921, now accepted",
            mutate: |root| observed_select_from(root, 921),
            base: structural,
            head: opaque,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: vec!["select-from"],
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "a case marked documented (D113), refused with 1003 and now accepted, is listed apart (D135)",
            mutate: |root| {
                observed_select_from(root, 1003);
                mark_select_from(root, MARK_DOCUMENTED);
            },
            base: structural,
            head: opaque,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: vec!["select-from"],
        },
        ParserChangeTestCase {
            description: "a case marked settled, refused with 1003 and now accepted, still fails",
            mutate: |root| {
                observed_select_from(root, 1003);
                mark_select_from(root, "settled reject (D133)");
            },
            base: structural,
            head: opaque,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: vec!["select-from"],
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "a positionless 1003 is no parse, so its rejection is not flagged",
            mutate: |root| observed_select_17(root, "semantic_error", 1003),
            base: structural,
            head: rejecting_select_17,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "receipts that disagree on the statement are not used",
            mutate: |root| {
                observed_select_17(root, "accepted", 0);
                let receipt: String = receipt_line(&select_17(), "syntax_error");
                append_receipt(
                    root,
                    &with_fields(
                        &receipt,
                        serde_json::json!({"errno": 1003, "query_id": "query-again", "run_id": "run-2"}),
                    ),
                );
            },
            base: structural,
            head: rejecting_select_17,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: Vec::new(),
        },
        ParserChangeTestCase {
            description: "an unchanged parser flags nothing",
            mutate: |root| observed_select_17(root, "accepted", 0),
            base: structural,
            head: structural,
            expected_parsed_cases_refused: Vec::new(),
            expected_syntax_error_cases_accepted: Vec::new(),
            expected_documented_cases_accepted: Vec::new(),
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        verified_tree(directory.path());
        (test_case.mutate)(directory.path());
        let diff: SnapshotDiff = diff_snapshots(
            &snapshot_with(directory.path(), test_case.base),
            &snapshot_with(directory.path(), test_case.head),
        );
        assert_eq!(
            (
                diff.parsed_cases_refused,
                diff.syntax_error_cases_accepted,
                diff.documented_cases_accepted
            ),
            (
                ids(&test_case.expected_parsed_cases_refused),
                ids(&test_case.expected_syntax_error_cases_accepted),
                ids(&test_case.expected_documented_cases_accepted),
            ),
            "{}",
            test_case.description
        );
    }
}
