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
            description: "REPLACE of a column the star's ILIKE removes is SQ0204 and exits 1",
            arguments: &["check"],
            sql: "SELECT * ILIKE '%_id' REPLACE (price * 2 AS price) FROM orders",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "error[SQ0204]",
        },
        CheckCommandTestCase {
            description: "SQ0203 points at the QUALIFY clause, not the statement's first line",
            arguments: &["check"],
            sql: "SELECT o.id, SUM(o.price) AS total, 'qualify' AS note\nFROM orders AS o\nGROUP BY o.id\nQUALIFY total > 3",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "--> <stdin>:4:1\n  |\n4 | QUALIFY total > 3\n  | ^^^^^^^^^^^^^^^^^ ",
        },
        CheckCommandTestCase {
            description: "SQ0201 points at INTERSECT ALL between its operands",
            arguments: &["check"],
            sql: "SELECT id\nFROM orders\n  INTERSECT ALL\nSELECT id FROM returns",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "--> <stdin>:3:3\n  |\n3 |   INTERSECT ALL\n  |   ^^^^^^^^^^^^^ ",
        },
        CheckCommandTestCase {
            description: "SQ0201 points at INTERSECT ALL between operands that have no location",
            arguments: &["check"],
            sql: "SELECT 1\nINTERSECT ALL\nSELECT 2",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "--> <stdin>:2:1\n  |\n2 | INTERSECT ALL\n  | ^^^^^^^^^^^^^ ",
        },
        CheckCommandTestCase {
            description: "SQ0201 leaves out the parentheses around parenthesised operands",
            arguments: &["check"],
            sql: "(SELECT id FROM orders) MINUS ALL (SELECT id FROM returns)",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "--> <stdin>:1:25\n",
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
