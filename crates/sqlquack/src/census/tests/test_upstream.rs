use crate::census::tests::helpers::{PARTIAL_SCAN, UPSTREAM_CHAIN, owned, upstream_summary};
use crate::census::tests::test_types::{UpstreamSummary, UpstreamTestCase};

#[test]
fn given_a_refused_upstream_model_when_chaining_then_only_schema_typed_readers_are_unit_testable() {
    let test_cases = [
        UpstreamTestCase {
            description: "a refused model the schema file types: its readers convert, labelled, \
                          and count as unit-testable but not strict",
            typed: &["fuzzy_names"],
            expected_summary: UpstreamSummary {
                convertible: 4,
                strict: 2,
                labelled: vec![
                    (
                        "name_report".to_owned(),
                        owned(&["upstream_refused:fuzzy_names"]),
                    ),
                    (
                        "name_summary".to_owned(),
                        owned(&["upstream_refused:fuzzy_names"]),
                    ),
                ],
                refused: owned(&["fuzzy_names"]),
                text_names_both: true,
            },
        },
        UpstreamTestCase {
            description: "a refused model with no known columns: its readers stay blocked",
            typed: &[],
            expected_summary: UpstreamSummary {
                convertible: 2,
                strict: 2,
                labelled: Vec::new(),
                refused: owned(&["fuzzy_names", "name_report", "name_summary"]),
                text_names_both: true,
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            upstream_summary(UPSTREAM_CHAIN, test_case.typed),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_partial_scan_when_a_model_reference_has_no_file_then_its_readers_are_not_strict() {
    let test_cases = [UpstreamTestCase {
        description: "a referenced model outside the scan, typed by the schema: its readers are \
                      unit-testable but not strict, labelled with the missing model",
        typed: &["sounds"],
        expected_summary: UpstreamSummary {
            convertible: 3,
            strict: 1,
            labelled: vec![
                (
                    "name_report".to_owned(),
                    owned(&["upstream_missing:__SQLQUACK_RELATION__SOUNDS"]),
                ),
                (
                    "name_summary".to_owned(),
                    owned(&["upstream_missing:__SQLQUACK_RELATION__SOUNDS"]),
                ),
            ],
            refused: Vec::new(),
            text_names_both: true,
        },
    }];
    for test_case in test_cases {
        assert_eq!(
            upstream_summary(PARTIAL_SCAN, test_case.typed),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}
