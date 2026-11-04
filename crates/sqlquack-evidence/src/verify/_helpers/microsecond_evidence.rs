//! Recordings whose nanosecond columns the connector truncated to microseconds.

use std::sync::Arc;

use arrow::array::{Array, ArrayRef, Time64NanosecondArray, TimestampNanosecondArray};
use arrow::datatypes::{DataType, TimeUnit};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use sqlquack_compare::comparison::main::compare_with_replay::compare_with_replay;
use sqlquack_compare::comparison::models::{RecordedResult, RowOrder};
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::models::ComparatorPolicy;

/// Nanoseconds per microsecond.
const NANOS_PER_MICRO: i64 = 1_000;

/// The largest fractional precision a microsecond Arrow unit holds.
const MICROSECOND_SCALE: u32 = 6;

/// How the connector may have dropped sub-microsecond digits: toward the past or toward zero.
#[derive(Clone, Copy)]
pub(crate) enum Truncation {
    /// Toward the past.
    Floor,
    /// Toward zero.
    TowardZero,
}

/// Sub-microsecond-scale columns stored in a coarser unit and not proven whole by the recorder.
pub(crate) fn truncated_columns(recorded: &RecordedResult) -> Vec<usize> {
    let fields = recorded.rows.schema.fields();
    let mut columns: Vec<usize> = Vec::new();
    for (index, column) in recorded.columns.iter().enumerate() {
        if column.sub_microsecond_checked {
            continue;
        }
        let declared = declared_scale(&column.snowflake_type).unwrap_or_default();
        let coarser = fields
            .get(index)
            .is_some_and(|field| coarser_than_nanoseconds(field.data_type()));
        if declared > MICROSECOND_SCALE && coarser {
            columns.push(index);
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

/// `replayed` with `columns`' sub-microsecond digits dropped, or `None` when it has none there.
pub(crate) fn without_sub_microseconds(
    replayed: &ResultSet,
    columns: &[usize],
    truncation: Truncation,
) -> Result<Option<ResultSet>, ArrowError> {
    let mut changed = false;
    let mut batches: Vec<RecordBatch> = Vec::new();
    for batch in &replayed.batches {
        let mut arrays: Vec<ArrayRef> = Vec::new();
        for (index, array) in batch.columns().iter().enumerate() {
            let (truncated, any) = match columns.contains(&index) {
                true => truncate(array, truncation),
                false => (Arc::clone(array), false),
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

fn truncate(array: &ArrayRef, truncation: Truncation) -> (ArrayRef, bool) {
    let drop = |nanos: i64| match truncation {
        Truncation::Floor => nanos - nanos.rem_euclid(NANOS_PER_MICRO),
        Truncation::TowardZero => nanos - nanos % NANOS_PER_MICRO,
    };
    if let Some(values) = array.as_any().downcast_ref::<TimestampNanosecondArray>() {
        let any = values
            .iter()
            .flatten()
            .any(|nanos| nanos % NANOS_PER_MICRO != 0);
        let dropped: TimestampNanosecondArray = values.unary(drop);
        return (Arc::new(dropped.with_timezone_opt(values.timezone())), any);
    }
    if let Some(values) = array.as_any().downcast_ref::<Time64NanosecondArray>() {
        let any = values
            .iter()
            .flatten()
            .any(|nanos| nanos % NANOS_PER_MICRO != 0);
        let dropped: Time64NanosecondArray = values.unary(drop);
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
    let (_, rest) = upper.split_once('(')?;
    rest.trim_end_matches(')')
        .chars()
        .try_fold(0_u32, |scale, digit| Some(scale * 10 + digit.to_digit(10)?))
}

fn coarser_than_nanoseconds(data_type: &DataType) -> bool {
    match data_type {
        DataType::Timestamp(unit, _) | DataType::Time64(unit) => *unit != TimeUnit::Nanosecond,
        DataType::Time32(_) => true,
        _ => false,
    }
}
