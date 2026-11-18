use crate::lint::tests::{helpers, test_types};

#[test]
fn given_production_function_depth_when_linting_then_bounded_parser_accepts_it() {
    let test_cases = [
        test_types::FunctionDepthTestCase {
            description: "production depth remains supported",
            depth: 65,
            max_function_call_depth: None,
            expected_diagnostic_count: 0,
        },
        test_types::FunctionDepthTestCase {
            description: "the default limit reads 256 nested calls",
            depth: 256,
            max_function_call_depth: None,
            expected_diagnostic_count: 0,
        },
        test_types::FunctionDepthTestCase {
            description: "a raised limit of 300 reads 300 nested calls",
            depth: 300,
            max_function_call_depth: Some(300),
            expected_diagnostic_count: 0,
        },
    ];
    for test_case in test_cases {
        let sql = helpers::nested_function_sql(test_case.depth);
        let diagnostics = helpers::diagnostics_within(&sql, test_case.max_function_call_depth)
            .unwrap_or_else(|error| {
                panic!("{}: {error}", test_case.description);
            });
        assert_eq!(
            diagnostics.len(),
            test_case.expected_diagnostic_count,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_excessive_function_depth_when_linting_then_complexity_guard_rejects_it() {
    let test_cases = [
        test_types::FunctionDepthFailureTestCase {
            description: "the default limit refuses 257 nested calls and says how to raise it",
            depth: 257,
            max_function_call_depth: None,
            expected_error_end: "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED: value 257 exceeds \
                configured limit 256; raise max_function_call_depth, up to 512",
        },
        test_types::FunctionDepthFailureTestCase {
            description: "a lowered limit of 10 refuses 11 nested calls",
            depth: 11,
            max_function_call_depth: Some(10),
            expected_error_end: "value 11 exceeds configured limit 10; raise \
                max_function_call_depth, up to 512",
        },
        test_types::FunctionDepthFailureTestCase {
            description: "a limit past the ceiling is refused, naming the ceiling",
            depth: 300,
            max_function_call_depth: Some(1000),
            expected_error_end: "max_function_call_depth: the function-call depth limit must be \
                between 1 and 512, SQLQuack's fixed nesting ceiling, not 1000",
        },
        test_types::FunctionDepthFailureTestCase {
            description: "at the ceiling of 512, 600 nested calls are refused, naming it as fixed",
            depth: 600,
            max_function_call_depth: Some(512),
            expected_error_end: "E_GUARD_NESTING_DEPTH_EXCEEDED: value 513 exceeds configured \
                limit 512; 512 is SQLQuack's fixed nesting ceiling, which no setting raises",
        },
    ];
    for test_case in test_cases {
        let sql = helpers::nested_function_sql(test_case.depth);
        let error = helpers::diagnostics_within(&sql, test_case.max_function_call_depth)
            .expect_err(test_case.description);
        assert!(
            error.ends_with(test_case.expected_error_end),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_a_call_depth_limit_when_parsing_sql_then_the_limit_applies() {
    let test_cases = [
        test_types::ParseSqlDepthTestCase {
            description: "a raised limit of 400 parses 300 nested calls",
            depth: 300,
            max_function_call_depth: Some(400),
            expected_outcome_end: "parsed",
        },
        test_types::ParseSqlDepthTestCase {
            description: "the default limit refuses 300 nested calls",
            depth: 300,
            max_function_call_depth: None,
            expected_outcome_end: "value 257 exceeds configured limit 256; raise \
                max_function_call_depth, up to 512",
        },
        test_types::ParseSqlDepthTestCase {
            description: "a limit past the ceiling is refused",
            depth: 300,
            max_function_call_depth: Some(1000),
            expected_outcome_end: "the function-call depth limit must be between 1 and 512, \
                SQLQuack's fixed nesting ceiling, not 1000",
        },
    ];
    for test_case in test_cases {
        let sql = helpers::nested_function_sql(test_case.depth);
        let outcome: String = crate::parse_sql(&sql, "duckdb", test_case.max_function_call_depth)
            .map_or_else(|error| error, |_| "parsed".to_owned());
        assert!(
            outcome.ends_with(test_case.expected_outcome_end),
            "{}: {outcome}",
            test_case.description
        );
    }
}
