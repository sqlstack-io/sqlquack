use crate::comparison::models::MismatchKind;
use crate::comparison::tests::helpers::{column, decimal, float64, int64, recordings_mismatch};
use crate::comparison::tests::test_types::RecordingsTestCase;

#[test]
fn given_two_recordings_of_a_case_when_auditing_then_any_drift_is_reported() {
    let test_cases = [
        RecordingsTestCase {
            description: "the same values in another physical type",
            stored: vec![column("N", "NUMBER(38,0)", int64(&[Some(7)]))],
            fresh: vec![column("N", "NUMBER(38,0)", decimal(&[Some(7)], 38, 0))],
            expected_mismatch: None,
        },
        RecordingsTestCase {
            description: "a changed declared type",
            stored: vec![column("N", "NUMBER(38,0)", int64(&[Some(7)]))],
            fresh: vec![column("N", "NUMBER(38,2)", decimal(&[Some(700)], 38, 2))],
            expected_mismatch: Some(MismatchKind::ColumnType),
        },
        RecordingsTestCase {
            description: "a float one ulp away",
            stored: vec![column("F", "FLOAT", float64(&[Some(0.1)]))],
            fresh: vec![column(
                "F",
                "FLOAT",
                float64(&[Some(f64::from_bits(0.1_f64.to_bits() + 1))]),
            )],
            expected_mismatch: Some(MismatchKind::Value),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            recordings_mismatch(&test_case.stored, &test_case.fresh),
            test_case.expected_mismatch,
            "{}",
            test_case.description
        );
    }
}
