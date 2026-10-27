use crate::verify::_helpers::judge::divergence_error;
use crate::verify::tests::helpers::{repository_policy, test_policy};
use crate::verify::tests::test_types::DivergenceErrorTestCase;

#[test]
fn given_a_duckdb_error_when_a_divergence_is_expected_then_only_its_own_errors_count() {
    let test_cases = [
        DivergenceErrorTestCase {
            description: "the guard's tagged raise counts",
            error: "Conversion Error: Could not convert string 'timestamp_range: beyond' to INT32",
            expected_matched: Some("the timestamp_range guard's raise"),
        },
        DivergenceErrorTestCase {
            description: "the guard's raise with its raise ID counts",
            error: "Conversion Error: Could not convert string \
                    'timestamp_range#0123456789ab: beyond' to INT32",
            expected_matched: Some("the timestamp_range guard's raise"),
        },
        DivergenceErrorTestCase {
            description: "a declared native error counts",
            error: "Conversion Error: Could not convert string '0001-01-01' to INT64\nLINE 1",
            expected_matched: Some(
                "DuckDB error matching `Conversion Error: Could not convert string '*' to INT64*`",
            ),
        },
        DivergenceErrorTestCase {
            description: "an unrelated DuckDB error does not count",
            error: "Binder Error: Referenced column \"x\" not found",
            expected_matched: None,
        },
        DivergenceErrorTestCase {
            description: "another divergence's guard does not count",
            error: "Conversion Error: Could not convert string 'numeric_range: overflow' to INT32",
            expected_matched: None,
        },
    ];
    let policy = test_policy();
    for test_case in test_cases {
        assert_eq!(
            divergence_error(test_case.error, "timestamp_range", &policy).as_deref(),
            test_case.expected_matched,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_the_repository_policy_when_duckdb_cannot_convert_text_to_int64_then_only_timestamps_count()
{
    let test_cases = [
        DivergenceErrorTestCase {
            description: "a timestamp outside the nanosecond range counts as timestamp_range",
            error: "Conversion Error: Could not convert string '0001-01-01 00:00:00' to INT64",
            expected_matched: Some(
                "DuckDB error matching `Conversion Error: Could not convert string '*-*-* *:*:*' to INT64`",
            ),
        },
        DivergenceErrorTestCase {
            description: "a timestamp with nanoseconds counts as well",
            error: "Conversion Error: Could not convert string '2262-04-11 23:47:16.854775807' to INT64",
            expected_matched: Some(
                "DuckDB error matching `Conversion Error: Could not convert string '*-*-* *:*:*' to INT64`",
            ),
        },
        DivergenceErrorTestCase {
            description: "an unrelated INT64 conversion error does not count",
            error: "Conversion Error: Could not convert string 'abc' to INT64",
            expected_matched: None,
        },
    ];
    let policy = repository_policy();
    for test_case in test_cases {
        assert_eq!(
            divergence_error(test_case.error, "timestamp_range", &policy).as_deref(),
            test_case.expected_matched,
            "{}",
            test_case.description
        );
    }
}
