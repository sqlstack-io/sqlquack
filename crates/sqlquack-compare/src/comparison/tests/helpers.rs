use std::sync::Arc;

use arrow::array::{
    ArrayRef, Date32Array, Decimal128Array, Float64Array, Int64Array, StringArray,
    TimestampMicrosecondArray, TimestampNanosecondArray,
};
use arrow::datatypes::{Field, Schema};
use arrow::record_batch::RecordBatch;
use sqlquack_policy::policy::models::{
    ColumnNameMatching, ComparatorPolicy, FloatUlps, NullOrder, TimestampTzComparison, TypeEquality,
};

use crate::comparison::main::compare_recordings::compare_recordings;
use crate::comparison::main::compare_with_replay::compare_with_replay;
use crate::comparison::models::{MismatchKind, RecordedColumn, RecordedResult, RowOrder};
use crate::comparison::tests::test_types::TestColumn;
use crate::values::models::ResultSet;

pub(crate) fn int64(values: &[Option<i64>]) -> ArrayRef {
    Arc::new(Int64Array::from(values.to_vec()))
}

pub(crate) fn decimal(values: &[Option<i128>], precision: u8, scale: i8) -> ArrayRef {
    Arc::new(
        Decimal128Array::from(values.to_vec())
            .with_precision_and_scale(precision, scale)
            .expect("valid decimal type"),
    )
}

pub(crate) fn float64(values: &[Option<f64>]) -> ArrayRef {
    Arc::new(Float64Array::from(values.to_vec()))
}

pub(crate) fn text(values: &[Option<&str>]) -> ArrayRef {
    Arc::new(StringArray::from(values.to_vec()))
}

pub(crate) fn date(values: &[Option<i32>]) -> ArrayRef {
    Arc::new(Date32Array::from(values.to_vec()))
}

pub(crate) fn timestamp_ns(values: &[Option<i64>], zone: &str) -> ArrayRef {
    Arc::new(TimestampNanosecondArray::from(values.to_vec()).with_timezone(zone))
}

pub(crate) fn timestamp_us(values: &[Option<i64>], zone: &str) -> ArrayRef {
    Arc::new(TimestampMicrosecondArray::from(values.to_vec()).with_timezone(zone))
}

pub(crate) fn column(
    name: &'static str,
    snowflake_type: &'static str,
    values: ArrayRef,
) -> TestColumn {
    TestColumn {
        name,
        snowflake_type,
        values,
    }
}

fn result_set(columns: &[TestColumn]) -> ResultSet {
    let schema = Arc::new(Schema::new(
        columns
            .iter()
            .map(|column| Field::new(column.name, column.values.data_type().clone(), true))
            .collect::<Vec<_>>(),
    ));
    let batch = RecordBatch::try_new(
        schema.clone(),
        columns.iter().map(|column| column.values.clone()).collect(),
    )
    .expect("test columns have equal lengths");
    ResultSet {
        schema,
        batches: vec![batch],
    }
}

fn recorded(columns: &[TestColumn]) -> RecordedResult {
    RecordedResult {
        columns: columns
            .iter()
            .map(|column| RecordedColumn {
                name: column.name.to_owned(),
                snowflake_type: column.snowflake_type.to_owned(),
            })
            .collect(),
        rows: result_set(columns),
    }
}

fn comparator(float64_ulps: u32) -> ComparatorPolicy {
    let equality = |snowflake: &str, duckdb: &str| TypeEquality {
        snowflake: snowflake.to_owned(),
        duckdb: duckdb.to_owned(),
        reason: "test".to_owned(),
    };
    ComparatorPolicy {
        column_names: ColumnNameMatching::CaseInsensitive,
        null_order: NullOrder::NullsLastOnAscFirstOnDesc,
        timestamp_tz: TimestampTzComparison::Instant,
        float_ulps: FloatUlps {
            float32: 0,
            float64: float64_ulps,
        },
        type_equality: vec![
            equality("NUMBER(p,s)", "DECIMAL(p,s)"),
            equality("FLOAT", "DOUBLE"),
            equality("VARCHAR(n)", "VARCHAR"),
            equality("DATE", "DATE"),
            equality("TIMESTAMP_TZ(p)", "TIMESTAMPTZ"),
        ],
    }
}

pub(crate) fn replay_mismatch(
    recorded_columns: &[TestColumn],
    replayed_columns: &[TestColumn],
    order: RowOrder,
    float64_ulps: u32,
) -> Option<MismatchKind> {
    compare_with_replay(
        &recorded(recorded_columns),
        &result_set(replayed_columns),
        order,
        &comparator(float64_ulps),
    )
    .err()
    .map(|mismatch| mismatch.kind)
}

pub(crate) fn recordings_mismatch(
    stored: &[TestColumn],
    fresh: &[TestColumn],
) -> Option<MismatchKind> {
    compare_recordings(&recorded(stored), &recorded(fresh), RowOrder::Ordered)
        .err()
        .map(|mismatch| mismatch.kind)
}
