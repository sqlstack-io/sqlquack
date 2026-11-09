//! TIME building block identifiers and the spellings text is read in.

/// Reading text as a TIME: `duckdb.text_to_time`.
pub const TEXT_TO_TIME: &str = "duckdb.text_to_time";
/// A TIME cut to a precision: `duckdb.truncate_time`.
pub const TRUNCATE_TIME: &str = "duckdb.truncate_time";
/// A TIME as Snowflake's default text: `duckdb.time_text`.
pub const TIME_TEXT: &str = "duckdb.time_text";
/// A TIME moved by clock units, wrapping at midnight: `duckdb.time_add_value`.
pub const TIME_ADD: &str = "duckdb.time_add_value";
/// The time of day of a naive timestamp: `duckdb.timestamp_time`.
pub const TIMESTAMP_TIME: &str = "duckdb.timestamp_time";
/// Clock unit boundaries crossed between two TIMEs: `duckdb.time_diff_value`.
pub const TIME_DIFF: &str = "duckdb.time_diff_value";
/// Nanoseconds in a day.
pub const NANOS_PER_DAY: i64 = 86_400_000_000_000;
/// Microseconds in a day.
pub const MICROS_PER_DAY: i64 = 86_400_000_000;
/// Nanoseconds in a second.
pub const NANOS_PER_SECOND: i64 = 1_000_000_000;
/// The message of the error text that is not a time raises.
pub const TIME_TEXT_MESSAGE: &str = "Time is not recognized";
/// Text read as a TIME without a format: hour, minute, second and fraction groups 1, 2, 4 and 6.
pub const AUTO_TIME_PATTERN: &str =
    "^ *([0-9]{1,2}):([0-9]{1,2})(:([0-9]{1,2})([.]([0-9]{1,9}))?)? *$";
/// Text read as `HH24:MI`: hour and minute groups 1 and 2.
pub const HOUR_MINUTE_PATTERN: &str = "^ *([0-9]{1,2}):([0-9]{2}) *$";
/// Text read as `HH24MI`: hour and minute groups 1 and 2.
pub const COMPACT_HOUR_MINUTE_PATTERN: &str = "^([0-9]{1,2})([0-9]{2})$";
/// Spellings recorded as not a time without a format.
pub const AUTO_TIME_REJECTED: &[&str] = &["^ *[0-9]{1,2}-[0-9]{1,2}-[0-9]{1,2} *$"];
/// Spellings recorded as not a time in `HH24:MI`.
pub const HOUR_MINUTE_REJECTED: &[&str] =
    &["^ *[0-9]{1,2}:[0-9]{2}:[0-9]{2} *$", "^ *[0-9]{3,4} *$"];
/// Spellings recorded as not a time in `HH24MI`.
pub const COMPACT_HOUR_MINUTE_REJECTED: &[&str] = &["^ *[0-9]{1,2}:[0-9]{2} *$"];
/// The divergence text read as a TIME with a sub-microsecond part raises.
pub const TIME_SUBMICRO: &str = "time_submicro";
/// The fractional digits of a microsecond.
pub const MICROSECOND_PRECISION: u8 = 6;
/// The message of the error text not in a TIME format raises.
pub const TIME_FORMAT_MESSAGE: &str = "Can't parse as time with the format";
/// Nanoseconds in a millisecond.
pub const NANOS_PER_MILLISECOND: i64 = 1_000_000;
/// A DATE or naive timestamp as a day-and-clock order key: `duckdb.date_timestamp_key`.
pub const DATE_TIMESTAMP_KEY: &str = "duckdb.date_timestamp_key";
/// Nanoseconds in a day.
pub const DAY_NANOSECONDS: i64 = 86_400_000_000_000;
/// Microseconds in a day.
pub const DAY_MICROSECONDS: i64 = 86_400_000_000;
/// The lambda parameter holding a TIME text read's pattern groups.
pub const TIME_GROUPS: &str = "time_groups";
/// The lambda parameter holding a TIME text read's nanoseconds after midnight.
pub const TIME_NANOS: &str = "time_nanos";
