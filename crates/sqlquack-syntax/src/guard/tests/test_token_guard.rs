use serde_json::error::Category;

use crate::guard::constants::DEFAULT_MAX_PARSER_DEPTH;
use crate::guard::models::ComplexityGuardOptions;
use crate::guard::tests::helpers::guard_errors;
use crate::guard::tests::helpers::guard_json_accepted;
use crate::guard::tests::helpers::limit_json_problems;
use crate::guard::tests::helpers::parsed_max_parser_depth;
use crate::guard::tests::helpers::round_tripped;
use crate::guard::tests::test_types::GuardJsonTestCase;
use crate::guard::tests::test_types::GuardKeyTestCase;
use crate::guard::tests::test_types::GuardLimitJsonTestCase;
use crate::guard::tests::test_types::GuardRoundTripTestCase;
use crate::guard::tests::test_types::GuardStatsTestCase;

#[test]
fn given_guard_json_when_parsing_then_parser_depth_takes_the_default_or_rejects_bad_values() {
    let test_cases = [
        GuardJsonTestCase {
            description: "an omitted limit takes the default",
            json: "{}",
            expected_max_parser_depth: Ok(Some(DEFAULT_MAX_PARSER_DEPTH)),
        },
        GuardJsonTestCase {
            description: "a negative limit is rejected",
            json: "{\"maxParserDepth\":-1}",
            expected_max_parser_depth: Err(Category::Data),
        },
        GuardJsonTestCase {
            description: "a fractional limit is rejected",
            json: "{\"maxParserDepth\":1.5}",
            expected_max_parser_depth: Err(Category::Data),
        },
        GuardJsonTestCase {
            description: "a boolean limit is rejected",
            json: "{\"maxParserDepth\":true}",
            expected_max_parser_depth: Err(Category::Data),
        },
        GuardJsonTestCase {
            description: "a string limit is rejected",
            json: "{\"maxParserDepth\":\"10\"}",
            expected_max_parser_depth: Err(Category::Data),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            parsed_max_parser_depth(test_case.json),
            test_case.expected_max_parser_depth,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_parser_depth_limit_when_round_tripping_json_then_the_limit_survives() {
    let test_cases = [
        GuardRoundTripTestCase {
            description: "the default limit survives",
            max_parser_depth: ComplexityGuardOptions::default().max_parser_depth,
            expected_max_parser_depth: Some(DEFAULT_MAX_PARSER_DEPTH),
        },
        GuardRoundTripTestCase {
            description: "a zero limit survives",
            max_parser_depth: Some(0),
            expected_max_parser_depth: Some(0),
        },
        GuardRoundTripTestCase {
            description: "a one limit survives",
            max_parser_depth: Some(1),
            expected_max_parser_depth: Some(1),
        },
        GuardRoundTripTestCase {
            description: "a large limit survives",
            max_parser_depth: Some(2048),
            expected_max_parser_depth: Some(2048),
        },
        GuardRoundTripTestCase {
            description: "no limit survives",
            max_parser_depth: None,
            expected_max_parser_depth: None,
        },
    ];

    for test_case in test_cases {
        let guard = ComplexityGuardOptions {
            max_parser_depth: test_case.max_parser_depth,
            ..Default::default()
        };
        assert_eq!(
            round_tripped(&guard).max_parser_depth,
            test_case.expected_max_parser_depth,
            "{}",
            test_case.description
        );
    }
}

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

#[test]
fn given_each_guard_limit_when_reading_json_then_bad_values_are_rejected_and_good_ones_kept() {
    let test_cases = [
        GuardLimitJsonTestCase {
            description: "maxParserDepth rejects non-integers and keeps zero, 128 and null",
            field: "maxParserDepth",
            expected_problems: vec![],
        },
        GuardLimitJsonTestCase {
            description: "maxInputBytes rejects non-integers and keeps zero, 128 and null",
            field: "maxInputBytes",
            expected_problems: vec![],
        },
        GuardLimitJsonTestCase {
            description: "maxTokens rejects non-integers and keeps zero, 128 and null",
            field: "maxTokens",
            expected_problems: vec![],
        },
        GuardLimitJsonTestCase {
            description: "maxAstNodes rejects non-integers and keeps zero, 128 and null",
            field: "maxAstNodes",
            expected_problems: vec![],
        },
        GuardLimitJsonTestCase {
            description: "maxAstDepth rejects non-integers and keeps zero, 128 and null",
            field: "maxAstDepth",
            expected_problems: vec![],
        },
        GuardLimitJsonTestCase {
            description: "maxParenthesisDepth rejects non-integers and keeps zero, 128 and null",
            field: "maxParenthesisDepth",
            expected_problems: vec![],
        },
        GuardLimitJsonTestCase {
            description: "maxFunctionCallDepth rejects non-integers and keeps zero, 128 and null",
            field: "maxFunctionCallDepth",
            expected_problems: vec![],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            limit_json_problems(test_case.field),
            test_case.expected_problems,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_guard_json_keys_when_reading_then_only_camel_case_names_are_accepted() {
    let test_cases = [GuardKeyTestCase {
        description: "a snake_case limit name is refused",
        json: r#"{"max_function_call_depth":128}"#,
        expected_accepted: false,
    }];

    for test_case in test_cases {
        assert_eq!(
            guard_json_accepted(test_case.json),
            test_case.expected_accepted,
            "{}",
            test_case.description
        );
    }
}
