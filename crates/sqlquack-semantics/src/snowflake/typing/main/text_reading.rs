//! `text_reading`: the type text takes beside other types in COALESCE, NVL, IFF and CASE.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::_helpers::numbers::unify_pair;
use crate::snowflake::typing::constants::{TEXT_AS_NUMBER, TEXT_READING};
use crate::snowflake::typing::models::SnowflakeType;

/// What text branches are read as beside other types, or `None` where no reading is recorded.
pub fn text_reading(types: &[SnowflakeType], log: &ProvenanceLog) -> Option<SnowflakeType> {
    let text = types.contains(&SnowflakeType::Varchar);
    let mut others = types
        .iter()
        .copied()
        .filter(|data_type| !matches!(data_type, SnowflakeType::Varchar | SnowflakeType::Null));
    let mut other = others.next()?;
    for data_type in others {
        other = match unify_pair(other, data_type) {
            Ok(unified) => unified,
            Err(_) => return None,
        };
    }
    let reading = match other {
        SnowflakeType::Number { .. } => TEXT_AS_NUMBER,
        SnowflakeType::Float => SnowflakeType::Float,
        SnowflakeType::TimestampNtz { precision: 9 } => other,
        _ => return None,
    };
    log.typing(TEXT_READING);
    text.then_some(reading)
}
