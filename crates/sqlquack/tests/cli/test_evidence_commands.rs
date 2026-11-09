use crate::helpers::{pending_flags, run_on_repository_cases, translation_statements};
use crate::test_types::{
    CommandOutcome, CommandTestCase, PendingFlagsTestCase, TranslationStatementTestCase,
};

#[test]
fn given_repository_policy_and_cases_when_running_evidence_commands_then_they_load_and_report() {
    let test_cases = [
        CommandTestCase {
            description: "pending writes pending case files",
            arguments: &["pending"],
            mention: "pending case files in",
            expected_outcome: CommandOutcome {
                exit_code: Some(0),
                wrote_pending_files: true,
                stderr_mentions: true,
            },
        },
        CommandTestCase {
            description: "verify validates the policy and cases and reports every case pending",
            arguments: &["verify"],
            mention: " unproven, 0 failed",
            expected_outcome: CommandOutcome {
                exit_code: Some(0),
                wrote_pending_files: false,
                stderr_mentions: true,
            },
        },
        CommandTestCase {
            description: "an audit sample with nothing recorded writes nothing",
            arguments: &["audit", "--sample", "50%", "--seed", "7"],
            mention: "sampled 0 recorded cases",
            expected_outcome: CommandOutcome {
                exit_code: Some(0),
                wrote_pending_files: false,
                stderr_mentions: true,
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_on_repository_cases(test_case.arguments, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_cases_declaring_errors_when_writing_pending_then_only_their_files_carry_the_flag() {
    let test_cases = [PendingFlagsTestCase {
        description: "compile_error marks only the declaring case, expected_error only the case \
                      tagged error, and a plain case carries neither",
        expected_flags: vec![
            ("relational/aliases.toml#plain_alias".to_owned(), None, None),
            (
                "relational/aliases.toml#reserved_alias".to_owned(),
                Some(true),
                None,
            ),
            (
                "relational/aliases.toml#zero_divisor".to_owned(),
                None,
                Some(true),
            ),
        ],
    }];
    for test_case in test_cases {
        assert_eq!(
            pending_flags(),
            test_case.expected_flags,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_recorded_cases_when_verify_writes_translation_statements_then_each_outcome_names_its_constructs()
 {
    let statements = translation_statements();
    let find = |id: &str| {
        let reference = format!("aggregate/aggregates_batch2.toml#{id}");
        statements
            .iter()
            .find(|statement| statement["reference"] == reference.as_str())
            .cloned()
            .unwrap_or_else(|| panic!("{reference} written"))
    };
    let strings = |value: &serde_json::Value| -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect()
    };
    let test_cases = [
        TranslationStatementTestCase {
            description: "matching rows",
            id: "any_value_all_null",
            expected_outcome: "matching",
            expected_codes: &[],
            expected_refused: &[],
            expected_use: "fn:ANY_VALUE",
        },
        TranslationStatementTestCase {
            description: "both raised the same kind of error",
            id: "object_agg_duplicate_key",
            expected_outcome: "matching_error",
            expected_codes: &[],
            expected_refused: &[],
            expected_use: "fn:OBJECT_AGG",
        },
        TranslationStatementTestCase {
            description: "a declared divergence's guard raised",
            id: "any_value_several_values",
            expected_outcome: "declared_divergence",
            expected_codes: &["divergence:any_value_choice"],
            expected_refused: &["fn:ANY_VALUE"],
            expected_use: "fn:ANY_VALUE",
        },
        TranslationStatementTestCase {
            description: "refused for one construct",
            id: "count_if_window_frames",
            expected_outcome: "refused",
            expected_codes: &["SQ0003"],
            expected_refused: &["fn:COUNT_IF"],
            expected_use: "node:window_function",
        },
        TranslationStatementTestCase {
            description: "refused as a whole statement",
            id: "any_value_with_aggregates",
            expected_outcome: "refused",
            expected_codes: &["SQ0004"],
            expected_refused: &["statement"],
            expected_use: "clause:group_by",
        },
        TranslationStatementTestCase {
            description: "a plain call keeps its shape",
            id: "count_if_comparison",
            expected_outcome: "matching",
            expected_codes: &[],
            expected_refused: &[],
            expected_use: "call:COUNT_IF",
        },
        TranslationStatementTestCase {
            description: "a call over a window keeps its shape",
            id: "count_if_window",
            expected_outcome: "matching",
            expected_codes: &[],
            expected_refused: &[],
            expected_use: "call_over:COUNT_IF",
        },
    ];
    for test_case in test_cases {
        let statement = find(test_case.id);

        assert_eq!(
            statement["outcome"], test_case.expected_outcome,
            "{}",
            test_case.description
        );
        assert_eq!(
            strings(&statement["codes"]),
            test_case.expected_codes,
            "{}",
            test_case.description
        );
        assert_eq!(
            strings(&statement["refused"]),
            test_case.expected_refused,
            "{}",
            test_case.description
        );
        assert!(
            strings(&statement["constructs"])
                .iter()
                .any(|key| key == test_case.expected_use),
            "{}: {statement}",
            test_case.description
        );
    }
    let aggregates = statements
        .iter()
        .filter(|statement| {
            statement["reference"]
                .as_str()
                .is_some_and(|reference| reference.starts_with("aggregate/"))
        })
        .count();
    assert_eq!(aggregates, 50);
}
