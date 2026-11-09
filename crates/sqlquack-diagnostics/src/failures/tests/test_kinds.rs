use crate::failures::constants::{
    DIVISION_BY_ZERO, JSON_NOT_PARSED, NUMBER_OUT_OF_RANGE, NUMBER_TEXT_OUT_OF_RANGE,
    STRING_TOO_LONG,
};
use crate::failures::main::engine_failure::engine_failure;
use crate::failures::main::tagged_message::tagged_message;
use crate::failures::models::{DuckdbFailure, ErrorKind};
use crate::failures::tests::helpers::untagged;
use crate::failures::tests::test_types::DuckdbMessageTestCase;

#[test]
fn given_duckdb_message_when_classifying_then_tag_or_fragment_gives_kind() {
    let test_cases = [
        DuckdbMessageTestCase {
            description: "a tagged message names its Snowflake error",
            message: format!(
                "Invalid Input Error: {}",
                tagged_message(DIVISION_BY_ZERO, "Division by zero: at row 1")
            ),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::DivisionByZero,
                snowflake: Some(DIVISION_BY_ZERO),
            }),
        },
        DuckdbMessageTestCase {
            description: "a tagged message with a raise ID names the same Snowflake error",
            message: "Conversion Error: Could not convert string \
                      'sqlquack:100051#0123456789ab: Division by zero' to INT32"
                .to_owned(),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::DivisionByZero,
                snowflake: Some(DIVISION_BY_ZERO),
            }),
        },
        DuckdbMessageTestCase {
            description: "a tag with an unknown number has no kind",
            message: "Invalid Input Error: sqlquack:1: unknown".to_owned(),
            expected_failure: None,
        },
        DuckdbMessageTestCase {
            description: "text that is not a number",
            message: "Conversion Error: Could not convert string 'x' to INT32".to_owned(),
            expected_failure: untagged(ErrorKind::NumberText),
        },
        DuckdbMessageTestCase {
            description: "text that is not a date",
            message: "Conversion Error: Could not convert string 'x' to DATE".to_owned(),
            expected_failure: untagged(ErrorKind::DateText),
        },
        DuckdbMessageTestCase {
            description: "a decimal overflow",
            message: "Out of Range Error: Overflow in multiplication of DECIMAL(38)".to_owned(),
            expected_failure: untagged(ErrorKind::NumericOverflow),
        },
        DuckdbMessageTestCase {
            description: "a catalog error has no kind",
            message: "Catalog Error: Table with name t does not exist".to_owned(),
            expected_failure: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            engine_failure(&test_case.message),
            test_case.expected_failure,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_tagged_snowflake_error_when_classifying_then_reads_back() {
    let test_cases = [
        DuckdbMessageTestCase {
            description: "DIVISION_BY_ZERO",
            message: tagged_message(DIVISION_BY_ZERO, "a message"),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::DivisionByZero,
                snowflake: Some(DIVISION_BY_ZERO),
            }),
        },
        DuckdbMessageTestCase {
            description: "NUMBER_OUT_OF_RANGE",
            message: tagged_message(NUMBER_OUT_OF_RANGE, "a message"),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::NumericOverflow,
                snowflake: Some(NUMBER_OUT_OF_RANGE),
            }),
        },
        DuckdbMessageTestCase {
            description: "NUMBER_TEXT_OUT_OF_RANGE",
            message: tagged_message(NUMBER_TEXT_OUT_OF_RANGE, "a message"),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::NumericOverflow,
                snowflake: Some(NUMBER_TEXT_OUT_OF_RANGE),
            }),
        },
        DuckdbMessageTestCase {
            description: "JSON_NOT_PARSED",
            message: tagged_message(JSON_NOT_PARSED, "a message"),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::JsonParse,
                snowflake: Some(JSON_NOT_PARSED),
            }),
        },
        DuckdbMessageTestCase {
            description: "STRING_TOO_LONG",
            message: tagged_message(STRING_TOO_LONG, "a message"),
            expected_failure: Some(DuckdbFailure {
                kind: ErrorKind::StringTooLong,
                snowflake: Some(STRING_TOO_LONG),
            }),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            engine_failure(&test_case.message),
            test_case.expected_failure,
            "{}: {}",
            test_case.description,
            test_case.message
        );
    }
}
