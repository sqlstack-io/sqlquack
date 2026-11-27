//! How Databricks SQL writes values in its results.

/// How the types start whose values compare as Databricks's text rendering.
pub const TEXT_TYPE_STARTS: &[&str] = &[
    "STRING",
    "ARRAY<",
    "MAP<",
    "STRUCT<",
    "VARIANT",
    "INTERVAL",
    "GEOMETRY",
    "GEOGRAPHY",
];
/// How a decimal type starts, before its precision and scale.
pub const DECIMAL_START: &str = "DECIMAL(";
/// How a TIME type with a precision starts.
pub const TIME_START: &str = "TIME(";
/// The most fraction digits a microsecond time holds.
pub const MICROSECOND_DIGITS: u8 = 6;
/// The Arrow zone of `TIMESTAMP` values, which the statement API writes in UTC.
pub const UTC_ZONE: &str = "UTC";
/// What ends the text of a `TIMESTAMP` value, and never a `TIMESTAMP_NTZ` value's.
pub const UTC_SUFFIX: char = 'Z';
/// A true BOOLEAN cell.
pub const TRUE_TEXT: &str = "true";
/// A false BOOLEAN cell.
pub const FALSE_TEXT: &str = "false";
/// What separates a decimal's fraction.
pub const DECIMAL_POINT: char = '.';
