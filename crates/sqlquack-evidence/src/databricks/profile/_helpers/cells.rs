//! Databricks results: each cell stored as the text the warehouse returned, decoded by its type.

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, AsArray, BinaryArray, BooleanArray, NullArray, StringArray,
    TimestampMillisecondArray,
};
use arrow::compute::kernels::cast::{CastOptions, cast_with_options};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sqlquack_compare::comparison::models::{RecordedColumn, RecordedResult};
use sqlquack_compare::values::models::ResultSet;

use crate::databricks::profile::constants::{
    DECIMAL_POINT, DECIMAL_START, FALSE_TEXT, MICROSECOND_DIGITS, TEXT_TYPE_STARTS, TIME_START,
    TRUE_TEXT, UTC_SUFFIX, UTC_ZONE,
};
use crate::errors::EvidenceError;
use crate::results::models::ResultMetadata;

/// How a Databricks column's cells decode.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Cells {
    /// Arrow's strict cast from text: integers, floats, dates and times.
    Cast(DataType),
    /// A decimal, its text holding no more fraction digits than its scale.
    Decimal(u8, i8),
    /// `true` or `false`.
    Boolean,
    /// Base64.
    Binary,
    /// Milliseconds; a zoned value is UTC and its text ends in `Z`, a naive one's never does.
    Timestamp {
        /// Whether the type is `TIMESTAMP` rather than `TIMESTAMP_NTZ`.
        zoned: bool,
    },
    /// Only NULLs.
    Void,
    /// Kept as Databricks's text rendering: strings, and nested, interval and spatial values.
    Text,
}

/// The recorded rows of a Databricks result, each text column decoded by its declared type.
pub(crate) fn text_result(
    metadata: &ResultMetadata,
    rows: &ResultSet,
) -> Result<RecordedResult, EvidenceError> {
    let label = &metadata.case_hash;
    let invalid = |message: String| EvidenceError::InvalidResult(format!("{label}: {message}"));
    let columns = &metadata.columns;
    if rows.schema.fields().len() != columns.len() {
        return Err(invalid(format!(
            "the Arrow file has {} columns, the metadata describes {}",
            rows.schema.fields().len(),
            columns.len()
        )));
    }
    let rows_read: usize = rows.batches.iter().map(RecordBatch::num_rows).sum();
    if metadata.row_count != Some(rows_read as u64) {
        return Err(invalid(format!(
            "the Arrow file has {rows_read} rows, the metadata says {:?}",
            metadata.row_count
        )));
    }
    let mut kinds: Vec<Cells> = Vec::with_capacity(columns.len());
    let mut fields: Vec<Field> = Vec::with_capacity(columns.len());
    for column in columns {
        let kind = cells(&column.type_name).map_err(invalid)?;
        fields.push(Field::new(&column.name, data_type(&kind), true));
        kinds.push(kind);
    }
    let schema = Arc::new(Schema::new(fields));
    let mut batches: Vec<RecordBatch> = Vec::with_capacity(rows.batches.len());
    for batch in &rows.batches {
        let mut arrays: Vec<ArrayRef> = Vec::with_capacity(kinds.len());
        for ((array, kind), column) in batch.columns().iter().zip(&kinds).zip(columns) {
            let decoded = decoded(array.as_ref(), kind).map_err(|message| {
                invalid(format!(
                    "column `{}` of type {}: {message}",
                    column.name, column.type_name
                ))
            })?;
            arrays.push(decoded);
        }
        let batch = RecordBatch::try_new(Arc::clone(&schema), arrays)
            .map_err(|error| invalid(error.to_string()))?;
        batches.push(batch);
    }
    let recorded_columns: Vec<RecordedColumn> = columns
        .iter()
        .map(|column| RecordedColumn {
            name: column.name.clone(),
            snowflake_type: column.type_name.clone(),
            sub_microsecond_checked: false,
        })
        .collect();
    Ok(RecordedResult {
        columns: recorded_columns,
        rows: ResultSet { schema, batches },
    })
}

/// How cells of the Databricks type `type_text` decode, or why they cannot.
fn cells(type_text: &str) -> Result<Cells, String> {
    let cast = match type_text {
        "TINYINT" => Some(DataType::Int8),
        "SMALLINT" => Some(DataType::Int16),
        "INT" => Some(DataType::Int32),
        "BIGINT" => Some(DataType::Int64),
        "FLOAT" => Some(DataType::Float32),
        "DOUBLE" => Some(DataType::Float64),
        "DATE" => Some(DataType::Date32),
        "TIME" => Some(DataType::Time64(TimeUnit::Microsecond)),
        _ => None,
    };
    if let Some(cast) = cast {
        return Ok(Cells::Cast(cast));
    }
    match type_text {
        "BOOLEAN" => return Ok(Cells::Boolean),
        "BINARY" => return Ok(Cells::Binary),
        "TIMESTAMP" => return Ok(Cells::Timestamp { zoned: true }),
        "TIMESTAMP_NTZ" => return Ok(Cells::Timestamp { zoned: false }),
        "VOID" => return Ok(Cells::Void),
        _ => {}
    }
    if let Some(arguments) = type_text
        .strip_prefix(DECIMAL_START)
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let unreadable = || format!("unreadable decimal type {type_text}");
        let (precision, scale) = arguments.split_once(',').ok_or_else(unreadable)?;
        let precision: u8 = precision.parse().map_err(|_| unreadable())?;
        let scale: i8 = scale.parse().map_err(|_| unreadable())?;
        return Ok(Cells::Decimal(precision, scale));
    }
    if let Some(precision) = type_text
        .strip_prefix(TIME_START)
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let precision: u8 = precision
            .parse()
            .map_err(|_| format!("unreadable time type {type_text}"))?;
        if precision > MICROSECOND_DIGITS {
            return Err(format!("{type_text} holds digits a microsecond does not"));
        }
        return Ok(Cells::Cast(DataType::Time64(TimeUnit::Microsecond)));
    }
    if TEXT_TYPE_STARTS
        .iter()
        .any(|start| type_text.starts_with(start))
    {
        return Ok(Cells::Text);
    }
    Err(format!("unknown Databricks type {type_text}"))
}

fn data_type(kind: &Cells) -> DataType {
    match kind {
        Cells::Cast(data_type) => data_type.clone(),
        Cells::Decimal(precision, scale) => DataType::Decimal128(*precision, *scale),
        Cells::Boolean => DataType::Boolean,
        Cells::Binary => DataType::Binary,
        Cells::Timestamp { zoned } => {
            DataType::Timestamp(TimeUnit::Millisecond, zoned.then(|| UTC_ZONE.into()))
        }
        Cells::Void => DataType::Null,
        Cells::Text => DataType::Utf8,
    }
}

fn decoded(array: &dyn Array, kind: &Cells) -> Result<ArrayRef, String> {
    let texts: &StringArray = array
        .as_string_opt::<i32>()
        .ok_or_else(|| "the column is not stored as text".to_owned())?;
    Ok(match kind {
        Cells::Cast(data_type) => strict_cast(texts, data_type)?,
        Cells::Decimal(_, scale) => {
            for text in texts.iter().flatten() {
                exact_decimal(text, *scale)?;
            }
            strict_cast(texts, &data_type(kind))?
        }
        Cells::Boolean => Arc::new(each(texts, boolean)?.into_iter().collect::<BooleanArray>()),
        Cells::Binary => {
            let bytes = each(texts, |text| {
                STANDARD
                    .decode(text)
                    .map_err(|error| format!("`{text}` is not base64: {error}"))
            })?;
            Arc::new(BinaryArray::from_iter(bytes))
        }
        Cells::Timestamp { zoned } => {
            for text in texts.iter().flatten() {
                if text.ends_with(UTC_SUFFIX) != *zoned {
                    return Err(format!("`{text}` is not written as the type's values are"));
                }
            }
            let naive = DataType::Timestamp(TimeUnit::Millisecond, None);
            let instants = strict_cast(texts, &naive)?;
            let millis: &TimestampMillisecondArray = instants.as_primitive();
            Arc::new(millis.clone().with_timezone_opt(zoned.then_some(UTC_ZONE)))
        }
        Cells::Void => {
            if texts.null_count() != texts.len() {
                return Err("a VOID column holds a value".to_owned());
            }
            Arc::new(NullArray::new(texts.len()))
        }
        Cells::Text => Arc::new(texts.clone()),
    })
}

/// `texts` cast to `data_type`, failing on any cell that does not parse.
fn strict_cast(texts: &StringArray, data_type: &DataType) -> Result<ArrayRef, String> {
    let options = CastOptions {
        safe: false,
        ..CastOptions::default()
    };
    cast_with_options(texts, data_type, &options).map_err(|error| error.to_string())
}

/// Each cell decoded by `decode`; NULL stays NULL.
fn each<T>(
    texts: &StringArray,
    decode: impl Fn(&str) -> Result<T, String>,
) -> Result<Vec<Option<T>>, String> {
    let mut values: Vec<Option<T>> = Vec::with_capacity(texts.len());
    for cell in texts {
        values.push(cell.map(&decode).transpose()?);
    }
    Ok(values)
}

fn boolean(text: &str) -> Result<bool, String> {
    match text {
        TRUE_TEXT => Ok(true),
        FALSE_TEXT => Ok(false),
        other => Err(format!("`{other}` is not a BOOLEAN value")),
    }
}

/// Fails unless `text` is plain decimal digits with at most `scale` after the point.
fn exact_decimal(text: &str, scale: i8) -> Result<(), String> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = digits.split_once(DECIMAL_POINT).unwrap_or((digits, ""));
    let plain = !whole.is_empty()
        && whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit());
    let fits = usize::try_from(scale).is_ok_and(|scale| fraction.len() <= scale);
    match plain && fits {
        true => Ok(()),
        false => Err(format!("`{text}` is not a decimal of scale {scale}")),
    }
}
