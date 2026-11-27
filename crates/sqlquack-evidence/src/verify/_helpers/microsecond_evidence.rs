//! Recordings whose columns the connector stored in a coarser unit than their type declares.

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, Time64NanosecondArray, TimestampMicrosecondArray, TimestampNanosecondArray,
};
use arrow::datatypes::{DataType, TimeUnit};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use sqlquack_compare::comparison::main::compare_with_replay::compare_with_replay;
use sqlquack_compare::comparison::models::{RecordedResult, RowOrder};
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::models::ComparatorPolicy;

/// Nanoseconds per microsecond.
const NANOS_PER_MICRO: i64 = 1_000;

/// The fractional precision of a time type that declares none, as Databricks's microsecond TIMESTAMP.
const UNDECLARED_SCALE: u32 = 6;

/// How the connector may have dropped sub-microsecond digits: toward the past or toward zero.
#[derive(Clone, Copy)]
pub(crate) enum Truncation {
    /// Toward the past.
    Floor,
    /// Toward zero.
    TowardZero,
}

/// Columns stored coarser than their declared precision and not proven whole, with the stored unit in nanoseconds.
pub(crate) fn truncated_columns(recorded: &RecordedResult) -> Vec<(usize, i64)> {
    let fields = recorded.rows.schema.fields();
    let mut columns: Vec<(usize, i64)> = Vec::new();
    for (index, column) in recorded.columns.iter().enumerate() {
        if column.sub_microsecond_checked {
            continue;
        }
        let Some(declared) = declared_scale(&column.snowflake_type) else {
            continue;
        };
        let stored = fields
            .get(index)
            .and_then(|field| stored_scale(field.data_type()));
        if let Some(stored) = stored.filter(|stored| declared > *stored) {
            columns.push((index, 10_i64.pow(9 - stored)));
        }
    }
    columns
}

/// Whether `replayed` matches `recorded` once the digits the recording truncated are dropped.
pub(crate) fn differs_below_microseconds(
    recorded: &RecordedResult,
    replayed: &ResultSet,
    order: RowOrder,
    comparator: &ComparatorPolicy,
) -> Result<bool, ArrowError> {
    let columns = truncated_columns(recorded);
    for truncation in [Truncation::Floor, Truncation::TowardZero] {
        let Some(truncated) = without_sub_microseconds(replayed, &columns, truncation)? else {
            continue;
        };
        match compare_with_replay(recorded, &truncated, order, comparator) {
            Ok(()) => return Ok(true),
            Err(_still_different) => {}
        }
    }
    Ok(false)
}

/// `replayed` with `columns`' digits below their stored unit dropped, or `None` when it has none there.
pub(crate) fn without_sub_microseconds(
    replayed: &ResultSet,
    columns: &[(usize, i64)],
    truncation: Truncation,
) -> Result<Option<ResultSet>, ArrowError> {
    let mut changed = false;
    let mut batches: Vec<RecordBatch> = Vec::new();
    for batch in &replayed.batches {
        let mut arrays: Vec<ArrayRef> = Vec::new();
        for (index, array) in batch.columns().iter().enumerate() {
            let unit = columns
                .iter()
                .find(|(column, _)| *column == index)
                .map(|(_, unit)| *unit);
            let (truncated, any) = match unit {
                Some(unit) => truncate(array, unit, truncation),
                None => (Arc::clone(array), false),
            };
            changed |= any;
            arrays.push(truncated);
        }
        batches.push(RecordBatch::try_new(batch.schema(), arrays)?);
    }
    Ok(changed.then(|| ResultSet {
        schema: Arc::clone(&replayed.schema),
        batches,
    }))
}

/// `array` without digits below `unit` nanoseconds, a microsecond array by its ticks, and whether it had any.
fn truncate(array: &ArrayRef, unit: i64, truncation: Truncation) -> (ArrayRef, bool) {
    let micro_unit = (unit / NANOS_PER_MICRO).max(1);
    let drop = |ticks: i64, unit: i64| match truncation {
        Truncation::Floor => ticks - ticks.rem_euclid(unit),
        Truncation::TowardZero => ticks - ticks % unit,
    };
    if let Some(values) = array.as_any().downcast_ref::<TimestampNanosecondArray>() {
        let any = values.iter().flatten().any(|nanos| nanos % unit != 0);
        let dropped: TimestampNanosecondArray = values.unary(|nanos| drop(nanos, unit));
        return (Arc::new(dropped.with_timezone_opt(values.timezone())), any);
    }
    if let Some(values) = array.as_any().downcast_ref::<TimestampMicrosecondArray>() {
        let any = values
            .iter()
            .flatten()
            .any(|micros| micros % micro_unit != 0);
        let dropped: TimestampMicrosecondArray = values.unary(|micros| drop(micros, micro_unit));
        return (Arc::new(dropped.with_timezone_opt(values.timezone())), any);
    }
    if let Some(values) = array.as_any().downcast_ref::<Time64NanosecondArray>() {
        let any = values.iter().flatten().any(|nanos| nanos % unit != 0);
        let dropped: Time64NanosecondArray = values.unary(|nanos| drop(nanos, unit));
        return (Arc::new(dropped), any);
    }
    (Arc::clone(array), false)
}

/// The fractional precision of a recorded TIME or TIMESTAMP type such as `TIMESTAMP_NTZ(9)`.
fn declared_scale(snowflake_type: &str) -> Option<u32> {
    let upper = snowflake_type.to_ascii_uppercase();
    if !upper.starts_with("TIME") {
        return None;
    }
    let Some((_, rest)) = upper.split_once('(') else {
        return Some(UNDECLARED_SCALE);
    };
    rest.trim_end_matches(')')
        .chars()
        .try_fold(0_u32, |scale, digit| Some(scale * 10 + digit.to_digit(10)?))
}

/// The fraction digits an Arrow time or timestamp unit holds, `None` for other types.
fn stored_scale(data_type: &DataType) -> Option<u32> {
    match data_type {
        DataType::Timestamp(unit, _) | DataType::Time64(unit) | DataType::Time32(unit) => {
            Some(match unit {
                TimeUnit::Second => 0,
                TimeUnit::Millisecond => 3,
                TimeUnit::Microsecond => 6,
                TimeUnit::Nanosecond => 9,
            })
        }
        _ => None,
    }
}
