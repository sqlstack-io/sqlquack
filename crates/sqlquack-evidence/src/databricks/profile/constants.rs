//! How Databricks SQL writes values in its results and names its errors.

use sqlquack_diagnostics::failures::models::ErrorKind;

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
/// The error class of a cast from malformed text, whose kind depends on the type cast to.
pub const CAST_INVALID_INPUT: &str = "CAST_INVALID_INPUT";
/// What precedes the type a malformed cast was to, in quotes, in the error's message.
pub const CAST_TARGET_MARKER: &str = "cannot be cast to \"";
/// The kind of a malformed cast by how the type cast to starts; the order puts longer starts first.
pub const TEXT_TARGET_KINDS: &[(&str, ErrorKind)] = &[
    ("TIMESTAMP", ErrorKind::TimestampText),
    ("DATE", ErrorKind::DateText),
    ("BOOLEAN", ErrorKind::BooleanText),
    ("TINYINT", ErrorKind::NumberText),
    ("SMALLINT", ErrorKind::NumberText),
    ("INT", ErrorKind::NumberText),
    ("BIGINT", ErrorKind::NumberText),
    ("DECIMAL", ErrorKind::NumberText),
    ("FLOAT", ErrorKind::NumberText),
    ("DOUBLE", ErrorKind::NumberText),
];
/// The kinds of the Databricks error classes a conversion may meet.
pub const ERROR_CLASS_KINDS: &[(&str, ErrorKind)] = &[
    ("DIVIDE_BY_ZERO", ErrorKind::DivisionByZero),
    ("REMAINDER_BY_ZERO", ErrorKind::DivisionByZero),
    ("ARITHMETIC_OVERFLOW", ErrorKind::NumericOverflow),
    ("NUMERIC_VALUE_OUT_OF_RANGE", ErrorKind::NumericOverflow),
    ("CAST_OVERFLOW", ErrorKind::NumericOverflow),
    ("INVALID_INDEX_OF_ZERO", ErrorKind::InvalidArgument),
    (
        "INVALID_ARRAY_INDEX_IN_ELEMENT_AT",
        ErrorKind::InvalidArgument,
    ),
];
