use std::sync::Arc;

use arrow::array::{TimestampMicrosecondArray, TimestampNanosecondArray};
use arrow::datatypes::{DataType, TimeUnit};
use sqlquack_compare::comparison::models::{RecordedColumn, RecordedResult, RowOrder};

use crate::results::models::WarehouseColumn;
use crate::verify::_helpers::microsecond_evidence::{
    Truncation, truncated_columns, without_sub_microseconds,
};
use crate::verify::main::judge_recording::judge_recording;
use crate::verify::tests::helpers::{floored_nanos, repository_policy, result_set};
use crate::verify::tests::test_types::{
    CheckedMicrosecondTestCase, MicrosecondEvidenceTestCase, SubMicrosecondMarkTestCase,
};

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
                sub_microsecond_checked: false,
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

#[test]
fn given_a_microsecond_recording_when_the_recorder_checked_it_then_it_compares_exactly() {
    let policy = repository_policy();
    let test_cases = [
        CheckedMicrosecondTestCase {
            description: "a checked column matches a replay with the same whole microseconds",
            checked: true,
            replayed_nanos: 1_000_001_000,
            expected_verdict: "Matches",
        },
        CheckedMicrosecondTestCase {
            description: "a checked column differs from a replay with sub-microsecond digits",
            checked: true,
            replayed_nanos: 1_000_001_123,
            expected_verdict: "Differs",
        },
        CheckedMicrosecondTestCase {
            description: "an unchecked column matches only at microseconds, even when equal",
            checked: false,
            replayed_nanos: 1_000_001_000,
            expected_verdict: "MatchesAtMicroseconds",
        },
        CheckedMicrosecondTestCase {
            description: "an unchecked column drops a replay's sub-microsecond digits",
            checked: false,
            replayed_nanos: 1_000_001_123,
            expected_verdict: "MatchesAtMicroseconds",
        },
        CheckedMicrosecondTestCase {
            description: "an unchecked column still differs by a whole microsecond",
            checked: false,
            replayed_nanos: 1_000_002_000,
            expected_verdict: "Differs",
        },
    ];
    for test_case in test_cases {
        let recorded = RecordedResult {
            columns: vec![RecordedColumn {
                name: "T".to_owned(),
                snowflake_type: "TIMESTAMP_NTZ(9)".to_owned(),
                sub_microsecond_checked: test_case.checked,
            }],
            rows: result_set(
                DataType::Timestamp(TimeUnit::Microsecond, None),
                Arc::new(TimestampMicrosecondArray::from(vec![1_000_001])),
            ),
        };
        let replayed = result_set(
            DataType::Timestamp(TimeUnit::Nanosecond, None),
            Arc::new(TimestampNanosecondArray::from(vec![
                test_case.replayed_nanos,
            ])),
        );
        let verdict = judge_recording(&recorded, &replayed, RowOrder::Ordered, &policy.comparator)
            .expect("the replay is judged");
        let kind = format!("{verdict:?}");
        assert_eq!(
            kind.split('(').next(),
            Some(test_case.expected_verdict),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_recorded_column_when_read_then_the_sub_microsecond_mark_is_optional() {
    let test_cases = [
        SubMicrosecondMarkTestCase {
            description: "the recorder writes the mark only on a checked column",
            json: r#"{"name": "T", "type_code": 8, "type_name": "TIMESTAMP_NTZ", "precision": 0, "scale": 9, "internal_size": null, "nullable": true, "sub_microsecond_checked": true}"#,
            expected_checked: true,
        },
        SubMicrosecondMarkTestCase {
            description: "a column without the mark, as every earlier recording has, is unchecked",
            json: r#"{"name": "T", "type_code": 8, "type_name": "TIMESTAMP_NTZ", "precision": 0, "scale": 9, "internal_size": null, "nullable": true}"#,
            expected_checked: false,
        },
    ];
    for test_case in test_cases {
        let column: WarehouseColumn =
            serde_json::from_str(test_case.json).expect("the recorded column reads");
        assert_eq!(
            column.sub_microsecond_checked, test_case.expected_checked,
            "{}",
            test_case.description
        );
    }
}
