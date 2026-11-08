//! `utc_kept`: whether a value chosen from others keeps the UTC offset they all hold.

use sqlquack_snowflake_types::typing::models::SnowflakeType;

/// Whether a TIMESTAMP_TZ `result` picked from `values` holds +00:00: each is one, or NULL.
pub fn utc_kept(values: &[(SnowflakeType, bool)], result: SnowflakeType) -> bool {
    matches!(result, SnowflakeType::TimestampTz { .. })
        && values
            .iter()
            .all(|(data_type, utc_offset)| match data_type {
                SnowflakeType::TimestampTz { .. } => *utc_offset,
                SnowflakeType::Null => true,
                _ => false,
            })
}
