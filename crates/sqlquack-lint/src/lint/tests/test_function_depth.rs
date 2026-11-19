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

#[test]
fn given_a_call_depth_limit_in_each_linted_dialect_when_linting_then_it_is_the_exact_limit() {
    let test_cases = [
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "a lowered limit of 3 lints 3 nested calls",
            depth: 3,
            max_function_call_depth: Some(3),
            expected_outcome_end: "linted",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "a lowered limit of 3 refuses 4 nested calls",
            depth: 4,
            max_function_call_depth: Some(3),
            expected_outcome_end: "value 4 exceeds configured limit 3; raise \
                max_function_call_depth, up to 512",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "a limit of 64 refuses 100 nested calls at the 65th",
            depth: 100,
            max_function_call_depth: Some(64),
            expected_outcome_end: "value 65 exceeds configured limit 64; raise \
                max_function_call_depth, up to 512",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "the default limit lints 256 nested calls",
            depth: 256,
            max_function_call_depth: None,
            expected_outcome_end: "linted",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "the default limit refuses 257 nested calls",
            depth: 257,
            max_function_call_depth: None,
            expected_outcome_end: "value 257 exceeds configured limit 256; raise \
                max_function_call_depth, up to 512",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "the ceiling of 512 lints 511 nested calls in a bare SELECT",
            depth: 511,
            max_function_call_depth: Some(512),
            expected_outcome_end: "linted",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "the ceiling of 512 refuses 512 nested calls, the statement being a level",
            depth: 512,
            max_function_call_depth: Some(512),
            expected_outcome_end: "value 513 exceeds configured limit 512. The maximum is 512. The \
                statement and every expression around the calls also count toward that limit, so at \
                512 a bare SELECT can nest at most 511 calls, and fewer fit inside an alias, \
                comparison or CTE.",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::aliased_calls_sql,
            description: "the ceiling of 512 lints 510 nested calls under an alias",
            depth: 510,
            max_function_call_depth: Some(512),
            expected_outcome_end: "linted",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::aliased_calls_sql,
            description: "the ceiling of 512 refuses 511 nested calls under an alias",
            depth: 511,
            max_function_call_depth: Some(512),
            expected_outcome_end: "value 513 exceeds configured limit 512. The maximum is 512. The \
                statement and every expression around the calls also count toward that limit, so at \
                512 a bare SELECT can nest at most 511 calls, and fewer fit inside an alias, \
                comparison or CTE.",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::compared_calls_sql,
            description: "the ceiling of 512 lints 510 nested calls in a comparison",
            depth: 510,
            max_function_call_depth: Some(512),
            expected_outcome_end: "linted",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::compared_calls_sql,
            description: "the ceiling of 512 refuses 511 nested calls in a comparison",
            depth: 511,
            max_function_call_depth: Some(512),
            expected_outcome_end: "value 513 exceeds configured limit 512. The maximum is 512. The \
                statement and every expression around the calls also count toward that limit, so at \
                512 a bare SELECT can nest at most 511 calls, and fewer fit inside an alias, \
                comparison or CTE.",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::cte_calls_sql,
            description: "the ceiling of 512 lints 509 nested calls in a CTE",
            depth: 509,
            max_function_call_depth: Some(512),
            expected_outcome_end: "linted",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::cte_calls_sql,
            description: "the ceiling of 512 refuses 510 nested calls in a CTE",
            depth: 510,
            max_function_call_depth: Some(512),
            expected_outcome_end: "value 513 exceeds configured limit 512. The maximum is 512. The \
                statement and every expression around the calls also count toward that limit, so at \
                512 a bare SELECT can nest at most 511 calls, and fewer fit inside an alias, \
                comparison or CTE.",
        },
        test_types::DialectDepthTestCase {
            sql: helpers::nested_function_sql,
            description: "the ceiling of 512 refuses 513 nested calls, naming it as fixed",
            depth: 513,
            max_function_call_depth: Some(512),
            expected_outcome_end: "value 513 exceeds configured limit 512; 512 is SQLQuack's \
                fixed nesting ceiling, which no setting raises",
        },
    ];
    for dialect in [
        "snowflake",
        "duckdb",
        "sqlite",
        "postgres",
        "bigquery",
        "databricks",
    ] {
        for test_case in &test_cases {
            let sql = (test_case.sql)(test_case.depth);
            let outcome: String = helpers::dialect_diagnostics_within(
                &sql,
                dialect,
                test_case.max_function_call_depth,
            )
            .map_or_else(|error| error, |_| "linted".to_owned());
            assert!(
                outcome.ends_with(test_case.expected_outcome_end),
                "{dialect}: {}: {outcome}",
                test_case.description
            );
        }
    }
}
