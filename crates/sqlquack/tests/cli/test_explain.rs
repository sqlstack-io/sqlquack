use crate::helpers::run_beside_policy;
use crate::test_types::ExplainTestCase;

#[test]
fn given_a_code_or_known_difference_when_explained_then_it_prints_or_the_command_exits_2() {
    let test_cases = [
        ExplainTestCase {
            description: "a code prints its title, explanation, help and example",
            arguments: &["explain", "SQ0203"],
            policy_file: None,
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
            arguments: &["explain", "e_guard_ast_depth_exceeded"],
            policy_file: None,
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "SQ0105 ast_depth_exceeded (formerly E_GUARD_AST_DEPTH_EXCEEDED): ",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "a code's snake_case name finds it",
            arguments: &["explain", "divergence_refused"],
            policy_file: None,
            expected_exit_code: Some(0),
            expected_stdout_mentions: &["SQ0008 divergence_refused: "],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "a helper macro prints its call, what it computes and its definition",
            arguments: &["explain", "sqlquack.variant_text_v1"],
            policy_file: None,
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "sqlquack.variant_text_v1(quack_site, quack_value): helper macro",
                "CREATE OR REPLACE MACRO sqlquack.json_escaped_v1(",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "an unregistered code exits 2 and says so",
            arguments: &["explain", "SQ9999"],
            policy_file: None,
            expected_exit_code: Some(2),
            expected_stdout_mentions: &[],
            expected_stderr_mention: "sqlquack explain: no diagnostic code, name, former name, known difference or helper macro is `SQ9999`",
        },
        ExplainTestCase {
            description: "a known difference prints its title, what differs, its details and how it fails",
            arguments: &["explain", "hash"],
            policy_file: None,
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "hash: HASH gives a different value in DuckDB\n\nDuckDB computes its own hash",
                "\n\nAnything that uses the hash value itself goes wrong:",
                "\nfine when: Joins, grouping,",
                "\nwrong when: Storing, comparing or sorting by the hash value itself:",
                "\nfails with: a different value\n",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "without a sqlquack.toml, only the built-in default's differences are allowed",
            arguments: &["explain", "--differences"],
            policy_file: None,
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "as the defaults treat them, with no ./sqlquack.toml:",
                "  json_fidelity             allowed by default  JSON",
                "  current_time              allowed by default  CURRENT_DATE",
                "  hash                      refused             HASH",
                "  random                    refused             RANDOM",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "a sqlquack.toml decides each difference, and project functions need a runtime",
            arguments: &["explain", "--differences"],
            policy_file: Some("sqlquack.toml"),
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "as ./sqlquack.toml treats them:",
                "  random                    allowed by sqlquack.toml  RANDOM",
                "  hash                      refused                   HASH",
                "  project_udf               refused                   Your Python",
            ],
            expected_stderr_mention: "",
        },
        ExplainTestCase {
            description: "every code lists with its name and title",
            arguments: &["explain", "--codes"],
            policy_file: None,
            expected_exit_code: Some(0),
            expected_stdout_mentions: &[
                "  SQ0001  parse_fidelity ",
                "  SQ0203  qualify_without_window ",
                "`sqlquack explain <code>` describes one.",
            ],
            expected_stderr_mention: "",
        },
    ];
    for test_case in test_cases {
        let (exit_code, stdout, stderr) =
            run_beside_policy(test_case.arguments, test_case.policy_file);
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
