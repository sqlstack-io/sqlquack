use crate::tokens::errors::ValidationError;
use crate::tokens::models::DialectType;
use crate::tokens::tests::test_types::DialectNameTestCase;
use crate::tokens::tests::test_types::ValidationLocationTestCase;

#[test]
fn given_a_validation_error_when_locating_it_then_line_column_and_span_are_kept() {
    let test_cases = [ValidationLocationTestCase {
        description: "location and span builders set every position",
        location: (1, 5),
        span: (Some(4), Some(10)),
        expected_position: (Some(1), Some(5), Some(4), Some(10)),
    }];

    for test_case in test_cases {
        let error = ValidationError::error("test", "E001")
            .with_location(test_case.location.0, test_case.location.1)
            .with_span(test_case.span.0, test_case.span.1);
        assert_eq!(
            (error.line, error.column, error.start, error.end),
            test_case.expected_position,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_dialect_name_when_parsing_it_then_case_does_not_matter_and_unknown_names_fail() {
    let test_cases = [
        DialectNameTestCase {
            description: "an unknown name is not a dialect",
            name: "invalid_dialect",
            expected_dialect: None,
        },
        DialectNameTestCase {
            description: "an upper-case name is accepted",
            name: "SNOWFLAKE",
            expected_dialect: Some(DialectType::Snowflake),
        },
        DialectNameTestCase {
            description: "a mixed-case name is accepted",
            name: "DuckDB",
            expected_dialect: Some(DialectType::DuckDB),
        },
        DialectNameTestCase {
            description: "a lower-case name is accepted",
            name: "duckdb",
            expected_dialect: Some(DialectType::DuckDB),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            test_case.name.parse::<DialectType>().ok(),
            test_case.expected_dialect,
            "{}",
            test_case.description
        );
    }
}
