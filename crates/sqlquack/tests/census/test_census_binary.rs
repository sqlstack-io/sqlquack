use crate::helpers::run_on_shop;
use crate::test_types::{BinaryOutcome, BinaryTestCase};

#[test]
fn given_sql_directory_when_running_census_binary_then_reports_on_the_right_streams() {
    let test_cases = [
        BinaryTestCase {
            description: "JSON goes to stdout and the completion message to stderr",
            arguments: &["census", "--json"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(0),
                files: Some(2),
                missing_rule: Some(2),
                completion_reported: true,
            },
        },
        BinaryTestCase {
            description: "an unknown dialect is a usage error",
            arguments: &["census", "--dialect", "unknown"],
            expected_outcome: BinaryOutcome {
                exit_code: Some(2),
                files: None,
                missing_rule: None,
                completion_reported: false,
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            run_on_shop(test_case.arguments),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
