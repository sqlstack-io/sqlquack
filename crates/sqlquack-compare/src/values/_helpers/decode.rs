//! Arrow arrays to canonical values.

use arrow::array::{Array, AsArray};
use arrow::datatypes::{
    DataType, Date32Type, Date64Type, Decimal128Type, Float32Type, Float64Type, Int8Type,
    Int16Type, Int32Type, Int64Type, Time32MillisecondType, Time32SecondType,
    Time64MicrosecondType, Time64NanosecondType, TimeUnit, TimestampMicrosecondType,
    TimestampMillisecondType, TimestampNanosecondType, TimestampSecondType, UInt8Type, UInt16Type,
    UInt32Type, UInt64Type,
};

use crate::values::models::CanonicalValue;

const MILLIS_PER_DAY: i64 = 86_400_000;

pub(crate) fn column_values(array: &dyn Array) -> Result<Vec<CanonicalValue>, String> {
    let values: Vec<Option<CanonicalValue>> = match array.data_type() {
        DataType::Null => vec![None; array.len()],
        DataType::Boolean => array
            .as_boolean()
            .iter()
            .map(|value| value.map(CanonicalValue::Boolean))
            .collect(),
        DataType::Int8 => integers(
            array
                .as_primitive::<Int8Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::Int16 => integers(
            array
                .as_primitive::<Int16Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::Int32 => integers(
            array
                .as_primitive::<Int32Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::Int64 => integers(
            array
                .as_primitive::<Int64Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::UInt8 => integers(
            array
                .as_primitive::<UInt8Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::UInt16 => integers(
            array
                .as_primitive::<UInt16Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::UInt32 => integers(
            array
                .as_primitive::<UInt32Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::UInt64 => integers(
            array
                .as_primitive::<UInt64Type>()
                .iter()
                .map(|value| value.map(i128::from)),
        ),
        DataType::Decimal128(_, scale) => array
            .as_primitive::<Decimal128Type>()
            .iter()
            .map(|value| value.map(|unscaled| number(unscaled, *scale)))
            .collect(),
        DataType::Float32 => array
            .as_primitive::<Float32Type>()
            .iter()
            .map(|value| value.map(float32))
            .collect(),
        DataType::Float64 => array
            .as_primitive::<Float64Type>()
            .iter()
            .map(|value| value.map(float64))
            .collect(),
        DataType::Utf8 => texts(array.as_string::<i32>().iter()),
        DataType::LargeUtf8 => texts(array.as_string::<i64>().iter()),
        DataType::Utf8View => texts(array.as_string_view().iter()),
        DataType::Binary => binaries(array.as_binary::<i32>().iter()),
        DataType::LargeBinary => binaries(array.as_binary::<i64>().iter()),
        DataType::BinaryView => binaries(array.as_binary_view().iter()),
        DataType::FixedSizeBinary(_) => binaries(array.as_fixed_size_binary().iter()),
        DataType::Date32 => array
            .as_primitive::<Date32Type>()
            .iter()
            .map(|value| value.map(|days| CanonicalValue::Date(i64::from(days))))
            .collect(),
        DataType::Date64 => array
            .as_primitive::<Date64Type>()
            .iter()
            .map(|value| {
                value.map(|millis| CanonicalValue::Date(millis.div_euclid(MILLIS_PER_DAY)))
            })
            .collect(),
        DataType::Time32(TimeUnit::Second) => times(
            array
                .as_primitive::<Time32SecondType>()
                .iter()
                .map(|value| value.map(i64::from)),
            1_000_000_000,
        ),
        DataType::Time32(TimeUnit::Millisecond) => times(
            array
                .as_primitive::<Time32MillisecondType>()
                .iter()
                .map(|value| value.map(i64::from)),
            1_000_000,
        ),
        DataType::Time64(TimeUnit::Microsecond) => {
            times(array.as_primitive::<Time64MicrosecondType>().iter(), 1_000)
        }
        DataType::Time64(TimeUnit::Nanosecond) => {
            times(array.as_primitive::<Time64NanosecondType>().iter(), 1)
        }
        DataType::Timestamp(unit, zone) => {
            let zone = zone.as_ref().map(ToString::to_string);
            let (raw, factor): (Vec<Option<i64>>, i128) = match unit {
                TimeUnit::Second => (
                    array.as_primitive::<TimestampSecondType>().iter().collect(),
                    1_000_000_000,
                ),
                TimeUnit::Millisecond => (
                    array
                        .as_primitive::<TimestampMillisecondType>()
                        .iter()
                        .collect(),
                    1_000_000,
                ),
                TimeUnit::Microsecond => (
                    array
                        .as_primitive::<TimestampMicrosecondType>()
                        .iter()
                        .collect(),
                    1_000,
                ),
                TimeUnit::Nanosecond => (
                    array
                        .as_primitive::<TimestampNanosecondType>()
                        .iter()
                        .collect(),
                    1,
                ),
            };
            raw.into_iter()
                .map(|value| {
                    value.map(|ticks| CanonicalValue::Timestamp {
                        nanos: i128::from(ticks) * factor,
                        zone: zone.clone(),
                    })
                })
                .collect()
        }
        other => {
            return Err(format!(
                "values of Arrow type {other} have no canonical form"
            ));
        }
    };
    Ok(values
        .into_iter()
        .map(|value| value.unwrap_or(CanonicalValue::Null))
        .collect())
}

pub(crate) fn number(unscaled: i128, scale: i8) -> CanonicalValue {
    let mut unscaled = unscaled;
    let mut scale = scale;
    while scale > 0 && unscaled % 10 == 0 {
        unscaled /= 10;
        scale -= 1;
    }
    while scale < 0 {
        match unscaled.checked_mul(10) {
            Some(widened) => {
                unscaled = widened;
                scale += 1;
            }
            None => break,
        }
    }
    CanonicalValue::Number { unscaled, scale }
}

fn integers(values: impl Iterator<Item = Option<i128>>) -> Vec<Option<CanonicalValue>> {
    values
        .map(|value| value.map(|unscaled| number(unscaled, 0)))
        .collect()
}

fn float32(value: f32) -> CanonicalValue {
    CanonicalValue::Float32(if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    })
}

fn float64(value: f64) -> CanonicalValue {
    CanonicalValue::Float64(if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    })
}

fn texts<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Vec<Option<CanonicalValue>> {
    values
        .map(|value| value.map(|text| CanonicalValue::Text(text.to_owned())))
        .collect()
}

fn binaries<'a>(values: impl Iterator<Item = Option<&'a [u8]>>) -> Vec<Option<CanonicalValue>> {
    values
        .map(|value| value.map(|bytes| CanonicalValue::Binary(bytes.to_vec())))
        .collect()
}

fn times(values: impl Iterator<Item = Option<i64>>, factor: i64) -> Vec<Option<CanonicalValue>> {
    values
        .map(|value| value.map(|ticks| CanonicalValue::Time(ticks * factor)))
        .collect()
}
