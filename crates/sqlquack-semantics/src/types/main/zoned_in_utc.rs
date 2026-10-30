//! `zoned_in_utc`: whether a cast to TIMESTAMP_TZ gives values holding the UTC offset.

use sqlquack_snowflake::typing::models::SnowflakeType;

/// Whether a cast to `to` gives +00:00 in the UTC session: naive values take it, zoned keep theirs.
pub fn zoned_in_utc((from, utc_offset): (SnowflakeType, bool), to: SnowflakeType) -> bool {
    matches!(to, SnowflakeType::TimestampTz { .. })
        && match from {
            SnowflakeType::Date | SnowflakeType::TimestampNtz { .. } => true,
            SnowflakeType::TimestampTz { .. } => utc_offset,
            _ => false,
        }
}
