//! The DuckDB type the policy pairs with each Snowflake type.

use crate::blocks::constants::{DUCKDB_FRACTIONAL_PRECISION, NANOSECOND_DIGITS, TYPE_NAME};
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

/// The DuckDB type the policy pairs with `data_type`, or why there is none.
pub(crate) fn duckdb_type(data_type: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    log.building_block(TYPE_NAME);
    match data_type {
        SnowflakeType::Number { precision, scale } => Ok(format!("DECIMAL({precision},{scale})")),
        SnowflakeType::Float => Ok("DOUBLE".to_owned()),
        SnowflakeType::Varchar => Ok("VARCHAR".to_owned()),
        SnowflakeType::Boolean => Ok("BOOLEAN".to_owned()),
        SnowflakeType::Date => Ok("DATE".to_owned()),
        SnowflakeType::Time { precision } if precision <= NANOSECOND_DIGITS => {
            Ok("TIME_NS".to_owned())
        }
        SnowflakeType::TimestampNtz { precision } if precision <= DUCKDB_FRACTIONAL_PRECISION => {
            Ok("TIMESTAMP".to_owned())
        }
        SnowflakeType::TimestampNtz { precision } if precision <= NANOSECOND_DIGITS => {
            Ok("TIMESTAMP_NS".to_owned())
        }
        SnowflakeType::TimestampTz { precision } | SnowflakeType::TimestampLtz { precision }
            if precision <= NANOSECOND_DIGITS =>
        {
            Ok("TIMESTAMPTZ".to_owned())
        }
        SnowflakeType::Time { .. }
        | SnowflakeType::TimestampNtz { .. }
        | SnowflakeType::TimestampTz { .. }
        | SnowflakeType::TimestampLtz { .. } => {
            Err(format!("the policy pairs no DuckDB type with {data_type}"))
        }
        SnowflakeType::SemiStructured | SnowflakeType::Null => {
            Err(format!("the policy pairs no DuckDB type with {data_type}"))
        }
    }
}
