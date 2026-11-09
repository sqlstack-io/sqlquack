use crate::helpers::{first_refusal_help, run_with_environment, run_with_input};
use crate::test_types::{CheckCommandTestCase, ColorTestCase, DivergenceHelpTestCase};

const PLAIN_CHECK: &str = "sqlquack check: checking…\n\
    error[SQ0203]: QUALIFY needs a window function\n \
    --> <stdin>:1:10\n  |\n1 | SELECT 1 QUALIFY TRUE\n  |          ^^^^^^^ QUALIFY is used \
    without a window function in the SELECT list or QUALIFY\n  |\n  = help: move the condition \
    to WHERE or HAVING, or filter on a window function's result\nsqlquack check: found 1 problem\n";

const STYLED_CHECK: &str = "sqlquack check: checking…\n\
    \x1b[1;31merror[SQ0203]\x1b[0m\x1b[1m: QUALIFY needs a window function\x1b[0m\n \
    \x1b[1;34m-->\x1b[0m <stdin>:1:10\n  \x1b[1;34m|\x1b[0m\n\x1b[1;34m1 |\x1b[0m SELECT 1 \
    QUALIFY TRUE\n  \x1b[1;34m|\x1b[0m \x1b[1;31m         ^^^^^^^ QUALIFY is used without a window \
    function in the SELECT list or QUALIFY\x1b[0m\n  \x1b[1;34m|\x1b[0m\n  \x1b[1;34m=\x1b[0m \
    \x1b[1;36mhelp\x1b[0m: move the condition to WHERE or HAVING, or filter on a window \
    function's result\nsqlquack check: \x1b[1;31mfound 1 problem\x1b[0m\n";

const PLAIN_CONVERTED: &str = "sqlquack convert: converting…\n\
    sqlquack convert: done, converted under the built-in default\n";

const STYLED_CONVERTED: &str = "sqlquack convert: converting…\n\
    sqlquack convert: \x1b[1;32mdone, converted\x1b[0m under the built-in default\n";

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
            description: "a timestamp type with a precision before a string is SQ0211 and exits 1",
            arguments: &["check"],
            sql: "SELECT TIMESTAMP_NTZ(3) '2026-01-02 03:04:05' AS v",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "error[SQ0211]",
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
            description: "a bare expression is no Snowflake statement: SQ0001 and exits 1",
            arguments: &["check"],
            sql: "CAST(1 AS INT)",
            expected_exit_code: Some(1),
            expected_stdout_prefix: "",
            expected_stderr_mention: "Expected a statement, not an expression",
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

#[test]
fn given_a_color_choice_when_checked_or_converted_then_only_a_colored_stream_is_styled() {
    let test_cases = [
        ColorTestCase {
            description: "a pipe is not a terminal, so auto prints plain text",
            arguments: &["check"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: PLAIN_CHECK,
        },
        ColorTestCase {
            description: "--color always styles a pipe",
            arguments: &["check", "--color", "always"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: STYLED_CHECK,
        },
        ColorTestCase {
            description: "CLICOLOR_FORCE styles a pipe under auto",
            arguments: &["check"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[("CLICOLOR_FORCE", "1")],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: STYLED_CHECK,
        },
        ColorTestCase {
            description: "FORCE_COLOR styles a pipe under auto",
            arguments: &["check"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[("FORCE_COLOR", "1")],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: STYLED_CHECK,
        },
        ColorTestCase {
            description: "an empty FORCE_COLOR forces nothing",
            arguments: &["check"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[("FORCE_COLOR", "")],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: PLAIN_CHECK,
        },
        ColorTestCase {
            description: "NO_COLOR wins over FORCE_COLOR and CLICOLOR_FORCE",
            arguments: &["check"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[
                ("NO_COLOR", "1"),
                ("FORCE_COLOR", "1"),
                ("CLICOLOR_FORCE", "1"),
            ],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: PLAIN_CHECK,
        },
        ColorTestCase {
            description: "--color never wins over CLICOLOR_FORCE",
            arguments: &["check", "--color", "never"],
            sql: "SELECT 1 QUALIFY TRUE",
            environment: &[("CLICOLOR_FORCE", "1")],
            expected_exit_code: Some(1),
            expected_stdout: "",
            expected_stderr: PLAIN_CHECK,
        },
        ColorTestCase {
            description: "converted SQL on a pipe is plain under auto",
            arguments: &["convert"],
            sql: "SELECT 'a' AS s",
            environment: &[],
            expected_exit_code: Some(0),
            expected_stdout: "SELECT 'a' AS \"S\"\n",
            expected_stderr: PLAIN_CONVERTED,
        },
        ColorTestCase {
            description: "forced colour highlights keywords, strings and identifiers",
            arguments: &["convert", "--color", "always"],
            sql: "SELECT 'a' AS s",
            environment: &[],
            expected_exit_code: Some(0),
            expected_stdout: "\x1b[35mSELECT\x1b[0m \x1b[32m'a'\x1b[0m \x1b[35mAS\x1b[0m \
                \x1b[36m\"S\"\x1b[0m\n",
            expected_stderr: STYLED_CONVERTED,
        },
        ColorTestCase {
            description: "NO_COLOR keeps converted SQL plain despite CLICOLOR_FORCE",
            arguments: &["convert"],
            sql: "SELECT 'a' AS s",
            environment: &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")],
            expected_exit_code: Some(0),
            expected_stdout: "SELECT 'a' AS \"S\"\n",
            expected_stderr: PLAIN_CONVERTED,
        },
        ColorTestCase {
            description: "the JSON report is never styled, even when forced",
            arguments: &["convert", "--json", "--color", "always"],
            sql: "SELECT 'a' AS s",
            environment: &[],
            expected_exit_code: Some(0),
            expected_stdout: "{\n  \"converted\": true,\n  \"profile\": \"the built-in default\",\n  \
                \"sql\": \"SELECT 'a' AS \\\"S\\\"\",\n  \"refusals\": [],\n  \"divergences\": {},\n  \
                \"assumptions\": [],\n  \"divergence_failures\": {},\n  \"notes\": [],\n  \
                \"warnings\": [],\n  \"python_functions\": [],\n  \"macros\": [],\n  \
                \"overrides\": []\n}\n",
            expected_stderr: STYLED_CONVERTED,
        },
    ];
    let directory = tempfile::tempdir().expect("temporary directory");
    for test_case in test_cases {
        let (exit_code, stdout, stderr) = run_with_environment(
            directory.path(),
            test_case.arguments,
            test_case.sql,
            test_case.environment,
        );
        assert_eq!(
            (exit_code, stdout.as_str(), stderr.as_str()),
            (
                test_case.expected_exit_code,
                test_case.expected_stdout,
                test_case.expected_stderr
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_uncompiled_dbt_jinja_when_converting_then_the_help_points_to_dbt_compile() {
    let test_cases = [
        DivergenceHelpTestCase {
            description: "a pasted model with Jinja expressions",
            sql: "select {{ dbt_utils.generate_surrogate_key(['order_id']) }} as k \
                  from {{ ref('stg_orders') }}",
            flags: &[],
            expected_help: "this looks like uncompiled dbt Jinja; run `dbt compile` and check the \
                SQL it writes under target/compiled",
        },
        DivergenceHelpTestCase {
            description: "a parse failure without Jinja keeps no help",
            sql: "SELECT a, FROM orders WHERE (b = 1",
            flags: &[],
            expected_help: "",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            first_refusal_help(test_case.sql, test_case.flags),
            test_case.expected_help,
            "{}",
            test_case.description
        );
    }
}
