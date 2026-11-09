//! Snowflake scalar types.

use std::fmt::{self, Display};

use serde::Serialize;

use crate::schemas::types::DialectType;

/// A Snowflake scalar type, as exact as the result metadata Snowflake reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum SnowflakeType {
    /// `NUMBER(precision, scale)`.
    Number {
        /// Total digits, 1 to 38.
        precision: u8,
        /// Digits after the point.
        scale: u8,
    },
    /// `FLOAT`, a 64-bit double.
    Float,
    /// `VARCHAR`; the declared length never changes a value.
    Varchar,
    /// `BOOLEAN`.
    Boolean,
    /// `DATE`.
    Date,
    /// `TIME(precision)`.
    Time {
        /// Fractional-second digits.
        precision: u8,
    },
    /// `TIMESTAMP_NTZ(precision)`.
    TimestampNtz {
        /// Fractional-second digits.
        precision: u8,
    },
    /// `TIMESTAMP_TZ(precision)`.
    TimestampTz {
        /// Fractional-second digits.
        precision: u8,
    },
    /// `TIMESTAMP_LTZ(precision)`.
    TimestampLtz {
        /// Fractional-second digits.
        precision: u8,
    },
    /// `VARIANT`, `OBJECT` or `ARRAY`.
    SemiStructured,
    /// An untyped `NULL` literal, which takes the type of its context.
    Null,
}

impl DialectType for SnowflakeType {
    fn is_semi_structured(&self) -> bool {
        *self == Self::SemiStructured
    }
}

impl SnowflakeType {
    /// Digits before the point of a NUMBER.
    pub fn leading_digits(self) -> Option<u8> {
        match self {
            Self::Number { precision, scale } => Some(precision.saturating_sub(scale)),
            _ => None,
        }
    }
}

impl Display for SnowflakeType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number { precision, scale } => write!(formatter, "NUMBER({precision},{scale})"),
            Self::Float => formatter.write_str("FLOAT"),
            Self::Varchar => formatter.write_str("VARCHAR"),
            Self::Boolean => formatter.write_str("BOOLEAN"),
            Self::Date => formatter.write_str("DATE"),
            Self::Time { precision } => write!(formatter, "TIME({precision})"),
            Self::TimestampNtz { precision } => write!(formatter, "TIMESTAMP_NTZ({precision})"),
            Self::TimestampTz { precision } => write!(formatter, "TIMESTAMP_TZ({precision})"),
            Self::TimestampLtz { precision } => write!(formatter, "TIMESTAMP_LTZ({precision})"),
            Self::SemiStructured => formatter.write_str("VARIANT"),
            Self::Null => formatter.write_str("NULL"),
        }
    }
}

/// A cast's target: its type and, for text, the longest value it accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CastTarget {
    /// The target type.
    pub data_type: SnowflakeType,
    /// For VARCHAR(n) and CHAR(n): `n`; longer values raise an error.
    pub max_length: Option<u32>,
}
