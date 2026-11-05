use crate::tokens::errors::Error;
use crate::tokens::errors::ValidationError;
use crate::tokens::models::DialectType;
use crate::tokens::models::Span;
use crate::tokens::tests::helpers::error_position;
use crate::tokens::tests::helpers::parse_error;
use crate::tokens::tests::test_types::DialectNameTestCase;
use crate::tokens::tests::test_types::ErrorPositionTestCase;
use crate::tokens::tests::test_types::ValidationLocationTestCase;

#[test]
fn given_an_error_when_reading_its_position_then_line_column_and_offsets_are_kept() {
    let test_cases = [
        ErrorPositionTestCase {
            description: "a parse error keeps its span and names it in the message",
            error: Error::parse(
                "test message",
                Span {
                    start: 20,
                    end: 25,
                    line: 5,
                    column: 10,
                },
            ),
            expected_position: (Some(5), Some(10), Some(20), Some(25)),
            expected_message_parts: &["line 5", "column 10", "test message"],
        },
        ErrorPositionTestCase {
            description: "a tokenize error keeps its span",
            error: Error::tokenize(
                "bad token",
                Span {
                    start: 15,
                    end: 20,
                    line: 3,
                    column: 7,
                },
            ),
            expected_position: (Some(3), Some(7), Some(15), Some(20)),
            expected_message_parts: &["bad token"],
        },
        ErrorPositionTestCase {
            description: "the parser reports an unexpected token at its position",
            error: parse_error("SELECT 1 + 2)"),
            expected_position: (Some(1), Some(14), Some(12), Some(13)),
            expected_message_parts: &["line 1"],
        },
        ErrorPositionTestCase {
            description: "an unterminated block comment fails at its opening, never parsing on",
            error: parse_error("SELECT /* never closed"),
            expected_position: (Some(1), Some(23), Some(7), Some(22)),
            expected_message_parts: &["Unterminated block comment"],
        },
    ];

    for test_case in test_cases {
        let message = test_case.error.to_string();
        assert_eq!(
            (
                error_position(&test_case.error),
                test_case
                    .expected_message_parts
                    .iter()
                    .all(|part| message.contains(part))
            ),
            (test_case.expected_position, true),
            "{}",
            test_case.description
        );
    }
}

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
