use crate::helpers::{pending_compile_errors, run_on_repository_cases};
use crate::test_types::{CommandOutcome, CommandTestCase, PendingFlagsTestCase};

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
fn given_a_case_declaring_compile_error_when_writing_pending_then_only_its_file_carries_it() {
    let test_cases = [PendingFlagsTestCase {
        description: "the declaring case's pending file carries compile_error, the other none",
        expected_flags: vec![
            ("relational/aliases.toml#plain_alias".to_owned(), None),
            (
                "relational/aliases.toml#reserved_alias".to_owned(),
                Some(true),
            ),
        ],
    }];
    for test_case in test_cases {
        assert_eq!(
            pending_compile_errors(),
            test_case.expected_flags,
            "{}",
            test_case.description
        );
    }
}
