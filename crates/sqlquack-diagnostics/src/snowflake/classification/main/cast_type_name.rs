//! `cast_type_name`: the base name a cast's target type is classified under.

use sqlquack_syntax::DataType;

use crate::snowflake::classification::_helpers::names::base_type_name;
use crate::snowflake::classification::constants::{
    LOCAL_TIME_ZONE_ALIASES, NO_TIME_ZONE_ALIASES, TIME_ZONE_ALIASES,
};

/// The upper-case base name of `data_type` in Snowflake's spelling: `NUMBER(38, 0)` is `NUMBER`.
pub fn cast_type_name(data_type: &DataType) -> String {
    let name = match data_type {
        DataType::Boolean | DataType::Bit { .. } => "BOOLEAN",
        DataType::TinyInt { .. } => "TINYINT",
        DataType::SmallInt { .. } => "SMALLINT",
        DataType::Int { .. } => "INT",
        DataType::BigInt { .. } => "BIGINT",
        DataType::Int128 => "INT128",
        DataType::UInt8 => "UTINYINT",
        DataType::UInt16 => "USMALLINT",
        DataType::UInt32 => "UINTEGER",
        DataType::UInt64 => "UBIGINT",
        DataType::UInt128 => "UINT128",
        DataType::Float { .. } => "FLOAT",
        DataType::Double { .. } => "DOUBLE",
        DataType::Decimal { .. } => "DECIMAL",
        DataType::Char { .. } => "CHAR",
        DataType::VarChar { .. } | DataType::Text | DataType::TextWithLength { .. } => "VARCHAR",
        DataType::String { .. } => "STRING",
        DataType::Binary { .. } => "BINARY",
        DataType::VarBinary { .. } => "VARBINARY",
        DataType::Blob => "BLOB",
        DataType::VarBit { .. } => "VARBIT",
        DataType::Date => "DATE",
        DataType::Time { .. } => "TIME",
        DataType::Timestamp { timezone: true, .. } => "TIMESTAMPTZ",
        DataType::Timestamp { .. } => "TIMESTAMP",
        DataType::Interval { .. } => "INTERVAL",
        DataType::Json | DataType::JsonB => "VARIANT",
        DataType::Uuid => "UUID",
        DataType::Array { .. } => "ARRAY",
        DataType::List { element_type } => return cast_type_name(element_type),
        DataType::Map { .. } => "MAP",
        DataType::Vector { .. } => "VECTOR",
        DataType::Object { .. } | DataType::Struct { .. } => "OBJECT",
        DataType::Enum { .. } => "ENUM",
        DataType::Set { .. } => "SET",
        DataType::Union { .. } => "UNION",
        DataType::Nullable { inner } | DataType::NotNull { inner } => return cast_type_name(inner),
        DataType::Custom { name } => return custom_type_name(name),
        DataType::Geometry { .. } => "GEOMETRY",
        DataType::Geography { .. } => "GEOGRAPHY",
        DataType::CharacterSet { .. } => "CHAR",
        DataType::Unknown
        | DataType::Vertica { .. }
        | DataType::Oracle { .. }
        | DataType::Hana { .. } => "UNKNOWN",
    };
    name.to_owned()
}

fn custom_type_name(name: &str) -> String {
    let base = base_type_name(name);
    let canonical = if NO_TIME_ZONE_ALIASES.contains(&base.as_str()) {
        NO_TIME_ZONE_ALIASES[0]
    } else if LOCAL_TIME_ZONE_ALIASES.contains(&base.as_str()) {
        LOCAL_TIME_ZONE_ALIASES[0]
    } else if TIME_ZONE_ALIASES.contains(&base.as_str()) {
        TIME_ZONE_ALIASES[0]
    } else {
        return base;
    };
    canonical.to_owned()
}
