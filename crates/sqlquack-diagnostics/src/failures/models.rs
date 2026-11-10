//! Error kinds, Snowflake error identities and the failure modes rules declare.

use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};

/// What kind of failure an error is, the same for Snowflake and DuckDB.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// A number beyond its type's range.
    NumericOverflow,
    /// Text that is not a number.
    NumberText,
    /// Text that is not a boolean.
    BooleanText,
    /// Text that is not a date.
    DateText,
    /// Text that is not a timestamp.
    TimestampText,
    /// Text that is not a time.
    TimeText,
    /// Division or modulo by zero.
    DivisionByZero,
    /// Text that is not JSON.
    JsonParse,
    /// A VARIANT value that cannot be cast to the target type.
    VariantCast,
    /// Text longer than its VARCHAR(n) type.
    StringTooLong,
    /// An argument a function does not accept.
    InvalidArgument,
    /// A statement that does not compile.
    Compilation,
}

impl ErrorKind {
    /// The kind's name.
    pub fn tag(self) -> &'static str {
        match self {
            Self::NumericOverflow => "numeric_overflow",
            Self::NumberText => "number_text",
            Self::BooleanText => "boolean_text",
            Self::DateText => "date_text",
            Self::TimestampText => "timestamp_text",
            Self::TimeText => "time_text",
            Self::DivisionByZero => "division_by_zero",
            Self::JsonParse => "json_parse",
            Self::VariantCast => "variant_cast",
            Self::StringTooLong => "string_too_long",
            Self::InvalidArgument => "invalid_argument",
            Self::Compilation => "compilation",
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.tag())
    }
}

/// A Snowflake error, identified by its number and SQLSTATE.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SnowflakeCode {
    /// The error number.
    pub errno: i64,
    /// The SQLSTATE.
    pub sqlstate: &'static str,
}

/// A way a rule's converted SQL may fail, and the Snowflake errors that mean the same failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FailureMode {
    /// The kind of failure.
    pub kind: ErrorKind,
    /// The Snowflake errors recorded for it.
    pub snowflake: &'static [SnowflakeCode],
}

/// What a DuckDB error message says: its kind, and the Snowflake error when a building block tagged it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DuckdbFailure {
    /// The kind of failure.
    pub kind: ErrorKind,
    /// The Snowflake error a building block raised it as, when tagged.
    pub snowflake: Option<SnowflakeCode>,
}
