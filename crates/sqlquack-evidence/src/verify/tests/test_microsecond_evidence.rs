use std::sync::Arc;

use arrow::array::TimestampNanosecondArray;
use arrow::datatypes::{DataType, TimeUnit};
use sqlquack_compare::comparison::models::{RecordedColumn, RecordedResult};

use crate::verify::_helpers::microsecond_evidence::{
    Truncation, truncated_columns, without_sub_microseconds,
};
use crate::verify::tests::helpers::{floored_nanos, result_set};
use crate::verify::tests::test_types::MicrosecondEvidenceTestCase;

#[test]
fn given_a_recording_in_a_coarser_unit_than_declared_when_checked_then_only_sub_microseconds_drop()
{
    let test_cases = [
        MicrosecondEvidenceTestCase {
            description: "a nanosecond column stored in microseconds drops a replay's nanoseconds",
            declared: "TIMESTAMP_NTZ(9)",
            stored_unit: TimeUnit::Microsecond,
            replayed_nanos: 1_000_000_123,
            expected_floored: Some(1_000_000_000),
        },
        MicrosecondEvidenceTestCase {
            description: "an instant before 1970 floors toward the past",
            declared: "TIMESTAMP_NTZ(9)",
            stored_unit: TimeUnit::Microsecond,
            replayed_nanos: -1,
            expected_floored: Some(-1_000),
        },
        MicrosecondEvidenceTestCase {
            description: "a replay without sub-microsecond digits leaves nothing to drop",
            declared: "TIMESTAMP_NTZ(9)",
            stored_unit: TimeUnit::Microsecond,
            replayed_nanos: 1_000_000_000,
            expected_floored: None,
        },
        MicrosecondEvidenceTestCase {
            description: "a nanosecond column stored in nanoseconds is full evidence",
            declared: "TIMESTAMP_NTZ(9)",
            stored_unit: TimeUnit::Nanosecond,
            replayed_nanos: 1_000_000_123,
            expected_floored: None,
        },
        MicrosecondEvidenceTestCase {
            description: "a microsecond column stored in microseconds is full evidence",
            declared: "TIMESTAMP_NTZ(6)",
            stored_unit: TimeUnit::Microsecond,
            replayed_nanos: 1_000_000_123,
            expected_floored: None,
        },
    ];
    for test_case in test_cases {
        let stored = result_set(
            DataType::Timestamp(test_case.stored_unit, None),
            arrow::array::new_null_array(&DataType::Timestamp(test_case.stored_unit, None), 1),
        );
        let recorded = RecordedResult {
            columns: vec![RecordedColumn {
                name: "T".to_owned(),
                snowflake_type: test_case.declared.to_owned(),
            }],
            rows: stored,
        };
        let replayed = result_set(
            DataType::Timestamp(TimeUnit::Nanosecond, None),
            Arc::new(TimestampNanosecondArray::from(vec![
                test_case.replayed_nanos,
            ])),
        );
        let columns = truncated_columns(&recorded);
        let floored = without_sub_microseconds(&replayed, &columns, Truncation::Floor)
            .expect("the replay's batches rebuild");
        assert_eq!(
            floored.as_ref().map(floored_nanos),
            test_case.expected_floored,
            "{}",
            test_case.description
        );
    }
}
