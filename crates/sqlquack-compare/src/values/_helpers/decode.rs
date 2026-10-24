//! Arrow arrays to canonical values.

use arrow::array::{Array, AsArray};
use arrow::datatypes::{
    ArrowPrimitiveType, DataType, Date32Type, Date64Type, Decimal128Type, Float32Type, Float64Type,
    Int8Type, Int16Type, Int32Type, Int64Type, Time32MillisecondType, Time32SecondType,
    Time64MicrosecondType, Time64NanosecondType, TimeUnit, TimestampMicrosecondType,
    TimestampMillisecondType, TimestampNanosecondType, TimestampSecondType, UInt8Type, UInt16Type,
    UInt32Type, UInt64Type,
};

use crate::values::models::CanonicalValue;

const MILLIS_PER_DAY: i64 = 86_400_000;
const NANOS_PER_SECOND: i64 = 1_000_000_000;
const NANOS_PER_MILLI: i64 = 1_000_000;
const NANOS_PER_MICRO: i64 = 1_000;

pub(crate) fn column_values(array: &dyn Array) -> Result<Vec<CanonicalValue>, String> {
    let zone: Option<String> = match array.data_type() {
        DataType::Timestamp(_, zone) => zone.as_ref().map(ToString::to_string),
        _ => None,
    };
    let instant = |ticks: i64, factor: i64| CanonicalValue::Timestamp {
        nanos: i128::from(ticks) * i128::from(factor),
        zone: zone.clone(),
    };
    Ok(match array.data_type() {
        DataType::Null => vec![CanonicalValue::Null; array.len()],
        DataType::Boolean => booleans(array),
        DataType::Int8 => primitive::<Int8Type>(array, |value| number(i128::from(value), 0)),
        DataType::Int16 => primitive::<Int16Type>(array, |value| number(i128::from(value), 0)),
        DataType::Int32 => primitive::<Int32Type>(array, |value| number(i128::from(value), 0)),
        DataType::Int64 => primitive::<Int64Type>(array, |value| number(i128::from(value), 0)),
        DataType::UInt8 => primitive::<UInt8Type>(array, |value| number(i128::from(value), 0)),
        DataType::UInt16 => primitive::<UInt16Type>(array, |value| number(i128::from(value), 0)),
        DataType::UInt32 => primitive::<UInt32Type>(array, |value| number(i128::from(value), 0)),
        DataType::UInt64 => primitive::<UInt64Type>(array, |value| number(i128::from(value), 0)),
        DataType::Decimal128(_, scale) => {
            primitive::<Decimal128Type>(array, |value| number(value, *scale))
        }
        DataType::Float32 => primitive::<Float32Type>(array, float32),
        DataType::Float64 => primitive::<Float64Type>(array, float64),
        DataType::Utf8 => texts(array.as_string::<i32>().iter()),
        DataType::LargeUtf8 => texts(array.as_string::<i64>().iter()),
        DataType::Utf8View => texts(array.as_string_view().iter()),
        DataType::Binary => binaries(array.as_binary::<i32>().iter()),
        DataType::LargeBinary => binaries(array.as_binary::<i64>().iter()),
        DataType::BinaryView => binaries(array.as_binary_view().iter()),
        DataType::FixedSizeBinary(_) => binaries(array.as_fixed_size_binary().iter()),
        DataType::Date32 => {
            primitive::<Date32Type>(array, |days| CanonicalValue::Date(i64::from(days)))
        }
        DataType::Date64 => primitive::<Date64Type>(array, |millis| {
            CanonicalValue::Date(millis.div_euclid(MILLIS_PER_DAY))
        }),
        DataType::Time32(TimeUnit::Second) => primitive::<Time32SecondType>(array, |ticks| {
            CanonicalValue::Time(i64::from(ticks) * NANOS_PER_SECOND)
        }),
        DataType::Time32(TimeUnit::Millisecond) => {
            primitive::<Time32MillisecondType>(array, |ticks| {
                CanonicalValue::Time(i64::from(ticks) * NANOS_PER_MILLI)
            })
        }
        DataType::Time64(TimeUnit::Microsecond) => {
            primitive::<Time64MicrosecondType>(array, |ticks| {
                CanonicalValue::Time(ticks * NANOS_PER_MICRO)
            })
        }
        DataType::Time64(TimeUnit::Nanosecond) => {
            primitive::<Time64NanosecondType>(array, CanonicalValue::Time)
        }
        DataType::Timestamp(TimeUnit::Second, _) => {
            primitive::<TimestampSecondType>(array, |ticks| instant(ticks, NANOS_PER_SECOND))
        }
        DataType::Timestamp(TimeUnit::Millisecond, _) => {
            primitive::<TimestampMillisecondType>(array, |ticks| instant(ticks, NANOS_PER_MILLI))
        }
        DataType::Timestamp(TimeUnit::Microsecond, _) => {
            primitive::<TimestampMicrosecondType>(array, |ticks| instant(ticks, NANOS_PER_MICRO))
        }
        DataType::Timestamp(TimeUnit::Nanosecond, _) => {
            primitive::<TimestampNanosecondType>(array, |ticks| instant(ticks, 1))
        }
        other => {
            return Err(format!(
                "values of Arrow type {other} have no canonical form"
            ));
        }
    })
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

fn primitive<T: ArrowPrimitiveType>(
    array: &dyn Array,
    convert: impl Fn(T::Native) -> CanonicalValue,
) -> Vec<CanonicalValue> {
    let typed = array.as_primitive::<T>();
    let mut values: Vec<CanonicalValue> = Vec::with_capacity(typed.len());
    for value in typed.iter() {
        values.push(value.map_or(CanonicalValue::Null, &convert));
    }
    values
}

fn booleans(array: &dyn Array) -> Vec<CanonicalValue> {
    let mut values: Vec<CanonicalValue> = Vec::with_capacity(array.len());
    for value in array.as_boolean().iter() {
        values.push(value.map_or(CanonicalValue::Null, CanonicalValue::Boolean));
    }
    values
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

fn texts<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Vec<CanonicalValue> {
    let mut canonical: Vec<CanonicalValue> = Vec::new();
    for value in values {
        canonical.push(value.map_or(CanonicalValue::Null, |text| {
            CanonicalValue::Text(text.to_owned())
        }));
    }
    canonical
}

fn binaries<'a>(values: impl Iterator<Item = Option<&'a [u8]>>) -> Vec<CanonicalValue> {
    let mut canonical: Vec<CanonicalValue> = Vec::new();
    for value in values {
        canonical.push(value.map_or(CanonicalValue::Null, |bytes| {
            CanonicalValue::Binary(bytes.to_vec())
        }));
    }
    canonical
}
