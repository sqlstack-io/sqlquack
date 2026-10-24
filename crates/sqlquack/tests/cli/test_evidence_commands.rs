use crate::helpers::run_on_repository_cases;
use crate::test_types::{CommandOutcome, CommandTestCase};

#[test]
fn given_repository_policy_and_cases_when_running_evidence_commands_then_they_load_and_report() {
    let test_cases = [
        CommandTestCase {
            description: "pending writes one file per seed case",
            arguments: &["pending"],
            mention: "9 pending case files in",
            expected_outcome: CommandOutcome {
                exit_code: Some(0),
                pending_files: 9,
                stderr_mentions: true,
            },
        },
        CommandTestCase {
            description: "verify validates the policy and cases and reports every case pending",
            arguments: &["verify"],
            mention: "9 pending, 0 failed",
            expected_outcome: CommandOutcome {
                exit_code: Some(0),
                pending_files: 0,
                stderr_mentions: true,
            },
        },
        CommandTestCase {
            description: "an audit sample with nothing recorded writes nothing",
            arguments: &["audit", "--sample", "50%", "--seed", "7"],
            mention: "sampled 0 recorded cases",
            expected_outcome: CommandOutcome {
                exit_code: Some(0),
                pending_files: 0,
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
