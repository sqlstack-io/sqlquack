use serde_json::json;
use sqlquack_compare::values::models::CanonicalValue;

use crate::databricks::journal::tests::helpers::{decoded_cell, outcome_of};
use crate::databricks::journal::tests::test_types::{CellTestCase, Outcome, OutcomeTestCase};

#[test]
fn given_a_recorded_cell_when_importing_then_it_reads_back_as_its_declared_type() {
    let number = |unscaled, scale| Some(CanonicalValue::Number { unscaled, scale });
    let test_cases = [
        CellTestCase {
            description: "an INT is a whole number",
            type_text: "INT",
            cell: Some("-7"),
            expected_value: number(-7, 0),
        },
        CellTestCase {
            description: "an INT outside 32 bits is refused",
            type_text: "INT",
            cell: Some("2147483648"),
            expected_value: None,
        },
        CellTestCase {
            description: "a NULL cell is NULL",
            type_text: "INT",
            cell: None,
            expected_value: Some(CanonicalValue::Null),
        },
        CellTestCase {
            description: "a DECIMAL keeps its digits",
            type_text: "DECIMAL(10,2)",
            cell: Some("2.50"),
            expected_value: number(25, 1),
        },
        CellTestCase {
            description: "a DECIMAL with more fraction digits than its scale is refused",
            type_text: "DECIMAL(10,2)",
            cell: Some("2.505"),
            expected_value: None,
        },
        CellTestCase {
            description: "a DOUBLE reads Infinity as Databricks writes it",
            type_text: "DOUBLE",
            cell: Some("Infinity"),
            expected_value: Some(CanonicalValue::Float64(f64::INFINITY.to_bits())),
        },
        CellTestCase {
            description: "a TIMESTAMP is a UTC instant to the millisecond",
            type_text: "TIMESTAMP",
            cell: Some("2016-07-30T12:34:56.789Z"),
            expected_value: Some(CanonicalValue::Timestamp {
                nanos: 1_469_882_096_789_000_000,
                zone: Some("UTC".to_owned()),
            }),
        },
        CellTestCase {
            description: "a TIMESTAMP written without its UTC mark is refused",
            type_text: "TIMESTAMP",
            cell: Some("2016-07-30T12:34:56.000"),
            expected_value: None,
        },
        CellTestCase {
            description: "a TIMESTAMP_NTZ is a wall-clock time without a zone",
            type_text: "TIMESTAMP_NTZ",
            cell: Some("1999-12-31T23:59:59.999"),
            expected_value: Some(CanonicalValue::Timestamp {
                nanos: 946_684_799_999_000_000,
                zone: None,
            }),
        },
        CellTestCase {
            description: "a DATE counts days from 1970, leap days included",
            type_text: "DATE",
            cell: Some("2024-02-29"),
            expected_value: Some(CanonicalValue::Date(19_782)),
        },
        CellTestCase {
            description: "a TIME(6) keeps microseconds",
            type_text: "TIME(6)",
            cell: Some("00:35:47.483647"),
            expected_value: Some(CanonicalValue::Time(2_147_483_647_000)),
        },
        CellTestCase {
            description: "a BINARY is base64",
            type_text: "BINARY",
            cell: Some("U3Bhcms="),
            expected_value: Some(CanonicalValue::Binary(b"Spark".to_vec())),
        },
        CellTestCase {
            description: "a BOOLEAN is only true or false",
            type_text: "BOOLEAN",
            cell: Some("yes"),
            expected_value: None,
        },
        CellTestCase {
            description: "an ARRAY compares as Databricks's text",
            type_text: "ARRAY<INT>",
            cell: Some("[1,null]"),
            expected_value: Some(CanonicalValue::Text("[1,null]".to_owned())),
        },
        CellTestCase {
            description: "a type the reader does not know is refused",
            type_text: "FILE",
            cell: Some("x"),
            expected_value: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            decoded_cell(test_case.type_text, test_case.cell),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_journal_line_when_importing_then_only_case_outcomes_are_recorded_once() {
    let failed = |message: &str| json!({"state": "FAILED", "error": {"error_code": "BAD_REQUEST", "message": message}});
    let pending = |reason: &str| Outcome {
        sqlstate: None,
        pending_reason: Some(reason.to_owned()),
        already_recorded: 0,
    };
    let test_cases = [
        OutcomeTestCase {
            description: "rows are recorded, and a second import leaves them",
            fields: json!({
                "state": "SUCCEEDED",
                "columns": [{"name": "r", "type_name": "INT", "type_text": "INT"}],
                "rows": [["1"]],
                "row_count": 1,
                "truncated": false,
            }),
            expected_outcome: Outcome {
                sqlstate: None,
                pending_reason: None,
                already_recorded: 1,
            },
        },
        OutcomeTestCase {
            description: "a data exception is the case's outcome",
            fields: failed("[ARITHMETIC_OVERFLOW] overflow. SQLSTATE: 22003"),
            expected_outcome: Outcome {
                sqlstate: Some("22003".to_owned()),
                pending_reason: None,
                already_recorded: 1,
            },
        },
        OutcomeTestCase {
            description: "a permission error depends on the account, not the case",
            fields: failed("[INSUFFICIENT_PERMISSIONS] denied. SQLSTATE: 42501"),
            expected_outcome: pending("SQLSTATE 42501"),
        },
        OutcomeTestCase {
            description: "an error naming no SQLSTATE is not a case outcome",
            fields: failed("Requested array size exceeds VM limit"),
            expected_outcome: pending("SQLSTATE unknown"),
        },
        OutcomeTestCase {
            description: "a raised user error is not a case outcome",
            fields: failed("[USER_RAISED_EXCEPTION] x SQLSTATE: P0001"),
            expected_outcome: pending("SQLSTATE P0001"),
        },
        OutcomeTestCase {
            description: "a truncated result is not recorded",
            fields: json!({
                "state": "SUCCEEDED",
                "columns": [{"name": "r", "type_name": "INT", "type_text": "INT"}],
                "rows": [["1"]],
                "row_count": 2,
                "truncated": true,
            }),
            expected_outcome: pending("holds 1 of Some(2) rows"),
        },
    ];

    for test_case in test_cases {
        let fragment = test_case.expected_outcome.pending_reason.as_deref();
        assert_eq!(
            outcome_of(&test_case.fields, fragment),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
