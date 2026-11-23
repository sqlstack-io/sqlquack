use sqlquack_syntax::ComplexityGuardOptions;

use crate::tokens::tests::helpers::guard_errors;
use crate::tokens::tests::test_types::GuardStatsTestCase;

#[test]
fn given_guard_options_when_checking_parser_stats_then_they_match_the_full_token_pass() {
    let test_cases = [
        GuardStatsTestCase {
            description: "the default options pass",
            options: ComplexityGuardOptions::default(),
            expected_error: Ok(()),
        },
        GuardStatsTestCase {
            description: "a token limit fails",
            options: ComplexityGuardOptions {
                max_tokens: Some(5),
                ..Default::default()
            },
            expected_error: Err("Parse error at line 1, column 21: E_GUARD_TOKEN_BUDGET_EXCEEDED: value 19 exceeds configured limit 5".to_string()),
        },
        GuardStatsTestCase {
            description: "a parenthesis depth limit fails",
            options: ComplexityGuardOptions {
                max_parenthesis_depth: Some(1),
                ..Default::default()
            },
            expected_error: Err("Parse error at line 1, column 20: E_GUARD_NESTING_DEPTH_EXCEEDED: value 2 exceeds configured limit 1".to_string()),
        },
        GuardStatsTestCase {
            description: "a function call depth of one is within the limit",
            options: ComplexityGuardOptions {
                max_function_call_depth: Some(1),
                ..Default::default()
            },
            expected_error: Ok(()),
        },
        GuardStatsTestCase {
            description: "the first exceeded limit is reported",
            options: ComplexityGuardOptions {
                max_parenthesis_depth: Some(2),
                max_function_call_depth: Some(1),
                ..Default::default()
            },
            expected_error: Err("Parse error at line 1, column 21: E_GUARD_NESTING_DEPTH_EXCEEDED: value 3 exceeds configured limit 2".to_string()),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            guard_errors(
                "SELECT outer(inner((value))), other(1, 2) FROM t",
                &test_case.options
            ),
            (test_case.expected_error.clone(), test_case.expected_error),
            "{}",
            test_case.description
        );
    }
}
