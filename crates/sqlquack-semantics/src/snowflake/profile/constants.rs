//! The values Snowflake's profile and lineage rules read.

/// How many distinct type spellings a thread keeps read before it starts over.
pub(crate) const SNOWFLAKE_KEPT_SPELLINGS: usize = 4096;
/// The length of Snowflake text declared without one.
pub(crate) const SNOWFLAKE_LONGEST_VARCHAR: u32 = 16_777_216;
/// What Snowflake's TIMESTAMP is under the default TIMESTAMP_TYPE_MAPPING.
pub(crate) const SNOWFLAKE_TIMESTAMP_TYPE: &str = "TIMESTAMP_NTZ";
/// Snowflake's semi-structured ARRAY type.
pub(crate) const SNOWFLAKE_ARRAY_TYPE: &str = "ARRAY";
/// Functions Snowflake types as COUNT.
pub(crate) const SNOWFLAKE_COUNT_FUNCTIONS: [&str; 1] = ["COUNT"];
/// Snowflake's names for DATEADD, whose result keeps its value's type.
pub(crate) const SNOWFLAKE_DATE_ADD_FUNCTIONS: [&str; 3] = ["DATEADD", "TIMEADD", "TIMESTAMPADD"];
/// Snowflake's names for DATEDIFF, written as a call when its unit is not a keyword.
pub(crate) const SNOWFLAKE_DATE_DIFF_FUNCTIONS: [&str; 3] =
    ["DATEDIFF", "TIMEDIFF", "TIMESTAMPDIFF"];
/// Date parts by which DATEADD moves a DATE and keeps it a DATE.
pub(crate) const SNOWFLAKE_DATE_UNITS: [&str; 5] = ["YEAR", "QUARTER", "MONTH", "WEEK", "DAY"];
/// The widest Snowflake NUMBER.
pub(crate) const SNOWFLAKE_MAX_PRECISION: u32 = 38;
/// The DATEDIFF unit whose TIME difference needs the wider recorded type.
pub(crate) const SNOWFLAKE_MICROSECOND_UNIT: &str = "MICROSECOND";
/// The ranking window functions.
pub(crate) const SNOWFLAKE_RANKING_FUNCTIONS: [&str; 3] = ["ROW_NUMBER", "RANK", "DENSE_RANK"];
/// The NUMBER Snowflake reads text as when a set operation combines it with a number.
pub(crate) const SNOWFLAKE_TEXT_NUMBER: (u32, u32) = (18, 5);
/// Time parts by which DATEADD moves a TIME and keeps it a TIME.
pub(crate) const SNOWFLAKE_TIME_UNITS: [&str; 6] = [
    "HOUR",
    "MINUTE",
    "SECOND",
    "MILLISECOND",
    "MICROSECOND",
    "NANOSECOND",
];
