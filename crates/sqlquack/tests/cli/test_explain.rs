use crate::helpers::run_with_input;
use crate::test_types::ExplainTestCase;

#[test]
fn given_a_code_when_explained_then_its_title_help_and_example_print_or_the_command_exits_2() {
    let test_cases = [
        ExplainTestCase {
            description: "a code prints its title, explanation, help and example",
            code: "SQ0203",
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "SQ0203 qualify_without_window: QUALIFY needs a window function\n\n",
                "Snowflake error 992",
                "help: move the condition to WHERE or HAVING",
                "example:\n    echo \"SELECT 1 QUALIFY TRUE\" | sqlquack check\n",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "a guard's former name in any case finds its code",
            code: "e_guard_ast_depth_exceeded",
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "SQ0105 ast_depth_exceeded (formerly E_GUARD_AST_DEPTH_EXCEEDED): ",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "a code's snake_case name finds it",
            code: "divergence_refused",
            expected_exit_code: Some(0),
            expected_stdout_mentions: &["SQ0008 divergence_refused: "],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "an unregistered code exits 2 and says so",
            code: "SQ9999",
            expected_exit_code: Some(2),
            expected_stdout_mentions: &[],
            expected_stderr_mention: "sqlquack explain: no diagnostic code, name or former name is `SQ9999`",
        },
    ];
    let directory = tempfile::tempdir().expect("temporary directory");
    for test_case in test_cases {
        let (exit_code, stdout, stderr) =
            run_with_input(directory.path(), &["explain", test_case.code], "");
        let missing: Vec<&str> = test_case
            .expected_stdout_mentions
            .iter()
            .copied()
            .filter(|mention| !stdout.contains(mention))
            .collect();
        assert_eq!(
            (
                exit_code,
                missing,
                stdout.is_empty(),
                stderr.contains(test_case.expected_stderr_mention)
            ),
            (
                test_case.expected_exit_code,
                Vec::<&str>::new(),
                test_case.expected_stdout_mentions.is_empty(),
                true
            ),
            "{}: {stdout}{stderr}",
            test_case.description
        );
    }
}
