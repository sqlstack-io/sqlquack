//! Snowflake's spellings of the types a cast is classified under.

/// Spellings of a timestamp without time zone a cast is classified under, the first one used.
pub const NO_TIME_ZONE_ALIASES: [&str; 2] = ["TIMESTAMPNTZ", "TIMESTAMP_NTZ"];
/// Spellings of a timestamp with local time zone, the first one used.
pub const LOCAL_TIME_ZONE_ALIASES: [&str; 2] = ["TIMESTAMPLTZ", "TIMESTAMP_LTZ"];
/// Spellings of a timestamp with time zone, the first one used.
pub const TIME_ZONE_ALIASES: [&str; 2] = ["TIMESTAMPTZ", "TIMESTAMP_TZ"];
