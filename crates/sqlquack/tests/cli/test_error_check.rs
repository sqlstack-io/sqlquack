use crate::helpers::verify_error_check;
use crate::test_types::{ErrorCheckOutcome, ErrorCheckTestCase};

#[test]
fn given_a_recorded_snowflake_error_when_verifying_then_duckdb_error_is_checked_against_it() {
    let test_cases = [
        ErrorCheckTestCase {
            description: "the recorded overflow agrees with the tagged DuckDB error",
            recorded_error: None,
            audited_message: None,
            arguments: &["--strict-errors"],
            expected_outcome: ErrorCheckOutcome {
                exit_code: Some(0),
                verdict: "passed".to_owned(),
                error_check: "agrees".to_owned(),
                snowflake_message: None,
            },
        },
        ErrorCheckTestCase {
            description: "another recorded errno is reported, but passes without --strict-errors",
            recorded_error: Some((100_051, "22012")),
            audited_message: None,
            arguments: &[],
            expected_outcome: ErrorCheckOutcome {
                exit_code: Some(0),
                verdict: "passed".to_owned(),
                error_check: "errno_mismatch".to_owned(),
                snowflake_message: None,
            },
        },
        ErrorCheckTestCase {
            description: "another recorded errno fails with --strict-errors",
            recorded_error: Some((100_051, "22012")),
            audited_message: None,
            arguments: &["--strict-errors"],
            expected_outcome: ErrorCheckOutcome {
                exit_code: Some(1),
                verdict: "passed".to_owned(),
                error_check: "errno_mismatch".to_owned(),
                snowflake_message: None,
            },
        },
        ErrorCheckTestCase {
            description: "a re-recording raising the same error supplies the message",
            recorded_error: None,
            audited_message: Some("Numeric value '1e40' is out of range"),
            arguments: &[],
            expected_outcome: ErrorCheckOutcome {
                exit_code: Some(0),
                verdict: "passed".to_owned(),
                error_check: "agrees".to_owned(),
                snowflake_message: Some("Numeric value '1e40' is out of range".to_owned()),
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            verify_error_check(
                (test_case.recorded_error, test_case.audited_message),
                test_case.arguments
            ),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
