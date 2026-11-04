use crate::helpers::run_with_input;
use crate::test_types::CheckCommandTestCase;

#[test]
fn given_snowflake_sql_when_checked_then_problems_are_coded_and_the_exit_status_says_so() {
    let test_cases = [
        CheckCommandTestCase {
            description: "sound SQL reports no problems and exits 0",
            arguments: &["check", "--json"],
            sql: "SELECT order_id FROM orders",
            expected_exit_code: Some(0),
            expected_stdout_prefix: "{\n  \"ok\": true,",
            expected_stderr_mention: "sqlquack check: done, no problems",
        },
        CheckCommandTestCase {
            description: "a construct Snowflake rejects is coded and exits 1",
            arguments: &["check", "--format", "json"],
            sql: "SELECT 1 QUALIFY TRUE",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "{\n  \"ok\": false,",
            expected_stderr_mention: "error[SQ0203]",
        },
        CheckCommandTestCase {
            description: "SQL that does not parse is SQ0001 and exits 1",
            arguments: &["check"],
            sql: "SELECT a,\n  FROM orders WHERE (b = 1\n",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "error[SQ0001]",
        },
        CheckCommandTestCase {
            description: "input holding no statement cannot be checked and exits 2",
            arguments: &["check", "-"],
            sql: "-- nothing here",
            expected_exit_code: Some(2),
            expected_stdout_prefix: "",
            expected_stderr_mention: "the input holds no SQL statement",
        },
    ];
    let directory = tempfile::tempdir().expect("temporary directory");
    for test_case in test_cases {
        let (exit_code, stdout, stderr) =
            run_with_input(directory.path(), test_case.arguments, test_case.sql);
        assert_eq!(
            (
                exit_code,
                stdout.starts_with(test_case.expected_stdout_prefix),
                stderr.contains(test_case.expected_stderr_mention)
            ),
            (test_case.expected_exit_code, true, true),
            "{}: {stdout}{stderr}",
            test_case.description
        );
    }
}
