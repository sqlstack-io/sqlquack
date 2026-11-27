//! The kinds of recorded Databricks errors, read from their error class.

use sqlquack_diagnostics::failures::models::ErrorKind;

use crate::databricks::profile::main::databricks_evidence::databricks_evidence;
use crate::databricks::profile::tests::test_types::FailureKindTestCase;
use crate::dialects::types::EvidenceProfile;
use crate::results::models::RecordedFailure;

#[test]
fn given_a_recorded_databricks_error_when_read_then_its_class_names_the_kind() {
    let test_cases = [
        FailureKindTestCase {
            description: "a malformed cast to INT is number text",
            message: Some(
                "[CAST_INVALID_INPUT] The value 'x' of the type \"STRING\" cannot be cast to \"INT\" because it is malformed.",
            ),
            expected_kind: Some(ErrorKind::NumberText),
        },
        FailureKindTestCase {
            description: "a malformed cast to TIMESTAMP_NTZ is timestamp text, not date text",
            message: Some(
                "[CAST_INVALID_INPUT] The value 'x' of the type \"STRING\" cannot be cast to \"TIMESTAMP_NTZ\" because it is malformed.",
            ),
            expected_kind: Some(ErrorKind::TimestampText),
        },
        FailureKindTestCase {
            description: "a remainder by zero is a division by zero",
            message: Some("[REMAINDER_BY_ZERO] Remainder by zero. SQLSTATE: 22012"),
            expected_kind: Some(ErrorKind::DivisionByZero),
        },
        FailureKindTestCase {
            description: "a class with a subclass reads by its class",
            message: Some("[ARITHMETIC_OVERFLOW.WITH_SUGGESTION] overflow. SQLSTATE: 22003"),
            expected_kind: Some(ErrorKind::NumericOverflow),
        },
        FailureKindTestCase {
            description: "a class no kind is known for",
            message: Some("[H3_INVALID_CELL_ID] -7 is not a valid H3 cell ID SQLSTATE: 22023"),
            expected_kind: None,
        },
        FailureKindTestCase {
            description: "an error recorded without its message",
            message: None,
            expected_kind: None,
        },
    ];
    for test_case in test_cases {
        let failure = RecordedFailure {
            errno: None,
            sqlstate: Some("22000".to_owned()),
            message: test_case.message.map(str::to_owned),
        };
        assert_eq!(
            databricks_evidence().failure_kind(&failure),
            Some(test_case.expected_kind),
            "{}",
            test_case.description
        );
    }
}
