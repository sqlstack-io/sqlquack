//! `rendered_type`: a cast target spelled as lineage reports types.

use sqlquack_syntax::DataType;

use crate::lineage::constants::{BOOLEAN_TYPE, TEXT_TYPE};

/// A cast target's spelling, parameters without spaces, when lineage knows the type.
pub(crate) fn rendered_type(data_type: &DataType) -> Option<String> {
    let text: String = match data_type {
        DataType::Boolean => BOOLEAN_TYPE.to_owned(),
        DataType::Date => "DATE".to_owned(),
        DataType::BigInt { .. } => "BIGINT".to_owned(),
        DataType::Int { .. } => "INT".to_owned(),
        DataType::Float { .. } => "FLOAT".to_owned(),
        DataType::Double { .. } => "DOUBLE".to_owned(),
        DataType::Decimal {
            precision: None, ..
        } => "DECIMAL".to_owned(),
        DataType::Decimal {
            precision: Some(precision),
            scale: None,
        } => format!("DECIMAL({precision})"),
        DataType::Decimal {
            precision: Some(precision),
            scale: Some(scale),
        } => format!("DECIMAL({precision},{scale})"),
        DataType::String { .. } => "STRING".to_owned(),
        DataType::Text => TEXT_TYPE.to_owned(),
        DataType::VarChar { length: None, .. } => TEXT_TYPE.to_owned(),
        DataType::VarChar {
            length: Some(length),
            ..
        } => format!("VARCHAR({length})"),
        DataType::Time { .. } => "TIME".to_owned(),
        DataType::Timestamp {
            timezone: false, ..
        } => "TIMESTAMP".to_owned(),
        DataType::Timestamp { timezone: true, .. } => "TIMESTAMPTZ".to_owned(),
        DataType::Custom { name } => name.replace(", ", ","),
        DataType::Array { element_type, .. } => format!("ARRAY({})", rendered_type(element_type)?),
        _ => return None,
    };
    Some(text)
}
