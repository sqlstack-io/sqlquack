//! DuckDB type names for the Arrow types DuckDB exports.

use arrow::datatypes::{DataType, TimeUnit};

pub(crate) fn duckdb_type_name(data_type: &DataType) -> String {
    match data_type {
        DataType::Boolean => "BOOLEAN".to_owned(),
        DataType::Int8 => "TINYINT".to_owned(),
        DataType::Int16 => "SMALLINT".to_owned(),
        DataType::Int32 => "INTEGER".to_owned(),
        DataType::Int64 => "BIGINT".to_owned(),
        DataType::UInt8 => "UTINYINT".to_owned(),
        DataType::UInt16 => "USMALLINT".to_owned(),
        DataType::UInt32 => "UINTEGER".to_owned(),
        DataType::UInt64 => "UBIGINT".to_owned(),
        DataType::Decimal128(precision, scale) => format!("DECIMAL({precision},{scale})"),
        DataType::Float32 => "FLOAT".to_owned(),
        DataType::Float64 => "DOUBLE".to_owned(),
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => "VARCHAR".to_owned(),
        DataType::Binary | DataType::LargeBinary | DataType::BinaryView => "BLOB".to_owned(),
        DataType::Date32 | DataType::Date64 => "DATE".to_owned(),
        DataType::Time64(TimeUnit::Nanosecond) => "TIME_NS".to_owned(),
        DataType::Time32(_) | DataType::Time64(_) => "TIME".to_owned(),
        DataType::Timestamp(_, Some(_)) => "TIMESTAMPTZ".to_owned(),
        DataType::Timestamp(TimeUnit::Second, None) => "TIMESTAMP_S".to_owned(),
        DataType::Timestamp(TimeUnit::Millisecond, None) => "TIMESTAMP_MS".to_owned(),
        DataType::Timestamp(TimeUnit::Microsecond, None) => "TIMESTAMP".to_owned(),
        DataType::Timestamp(TimeUnit::Nanosecond, None) => "TIMESTAMP_NS".to_owned(),
        other => format!(
            "ARROW_{}",
            other
                .to_string()
                .to_ascii_uppercase()
                .replace([' ', '(', ')', ','], "_")
        ),
    }
}
