//! `format_type`: a type as PostgreSQL's `format_type` function names it.

use sqlquack_syntax::DataType;

use crate::parse::constants::{CATALOGUE_SCHEMA, MAX_SECOND_PRECISION};

/// `name(n)`, or `name` without a modifier.
fn sized(name: &str, size: Option<u32>) -> String {
    match size {
        Some(size) => format!("{name}({size})"),
        None => name.to_owned(),
    }
}

/// `time` or `timestamp` with its precision, which PostgreSQL caps at six digits.
fn temporal(name: &str, precision: Option<u32>, timezone: bool) -> String {
    let zone: &str = if timezone { "with" } else { "without" };
    let capped: Option<u32> = precision.map(|digits| digits.min(MAX_SECOND_PRECISION));
    format!("{} {zone} time zone", sized(name, capped))
}

/// PostgreSQL 18's `format_type` name for `data_type`; none when PostgreSQL has no spelling for it.
pub fn format_type(data_type: &DataType) -> Option<String> {
    Some(match data_type {
        DataType::Int { .. } => "integer".to_owned(),
        DataType::SmallInt { .. } => "smallint".to_owned(),
        DataType::BigInt { .. } => "bigint".to_owned(),
        DataType::Float { .. } => "real".to_owned(),
        DataType::Double { .. } => "double precision".to_owned(),
        DataType::Decimal {
            precision: Some(precision),
            scale,
        } => format!("numeric({precision},{})", scale.unwrap_or(0)),
        DataType::Decimal { .. } => "numeric".to_owned(),
        DataType::Boolean => "boolean".to_owned(),
        DataType::Bit { length } => sized("bit", Some(length.unwrap_or(1))),
        DataType::VarBit { length } => sized("bit varying", *length),
        DataType::Char { length } => sized("character", Some(length.unwrap_or(1))),
        DataType::VarChar { length, .. } => sized("character varying", *length),
        DataType::Text => "text".to_owned(),
        DataType::Date => "date".to_owned(),
        DataType::Time {
            precision,
            timezone,
        } => temporal("time", *precision, *timezone),
        DataType::Timestamp {
            precision,
            timezone,
        } => temporal("timestamp", *precision, *timezone),
        DataType::Interval { unit, to } => {
            let fields: Vec<String> = [unit, to]
                .into_iter()
                .flatten()
                .map(|field| field.to_lowercase())
                .collect();
            match fields.as_slice() {
                [] => "interval".to_owned(),
                [only] => format!("interval {only}"),
                [from, to, ..] => format!("interval {from} to {to}"),
            }
        }
        DataType::Json => "json".to_owned(),
        DataType::JsonB => "jsonb".to_owned(),
        DataType::Uuid => "uuid".to_owned(),
        DataType::Array { element_type, .. } => match element_type.as_ref() {
            DataType::Array { .. } => format_type(element_type)?,
            element => format!("{}[]", format_type(element)?),
        },
        DataType::Custom { name } => name
            .strip_prefix(CATALOGUE_SCHEMA)
            .and_then(|rest| rest.strip_prefix('.'))
            .unwrap_or(name)
            .to_owned(),
        _ => return None,
    })
}
