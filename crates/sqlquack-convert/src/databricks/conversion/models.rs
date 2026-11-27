//! Databricks conversion results and the types conversion tracks.

use serde::Serialize;
use sqlquack_diagnostics::provenance::models::Provenance;

/// A Databricks SQL type a converted value is known to have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabricksType {
    /// `BOOLEAN`.
    Boolean,
    /// `TINYINT`, a signed 8-bit integer.
    TinyInt,
    /// `SMALLINT`, a signed 16-bit integer.
    SmallInt,
    /// `INT`, a signed 32-bit integer.
    Int,
    /// `BIGINT`, a signed 64-bit integer.
    BigInt,
    /// `FLOAT`, a 4-byte float.
    Float,
    /// `DOUBLE`, an 8-byte float.
    Double,
    /// `DECIMAL(p, s)`.
    Decimal {
        /// Total digits, 1 to 38.
        precision: u8,
        /// Digits after the point.
        scale: u8,
    },
    /// `STRING`.
    String,
    /// `DATE`.
    Date,
    /// `TIMESTAMP`, an instant shown in the session time zone, pinned to UTC.
    Timestamp,
    /// `TIMESTAMP_NTZ`, a wall-clock time without a zone.
    TimestampNtz,
    /// The type of a bare `NULL`, `VOID`.
    Null,
}

impl DatabricksType {
    /// The DuckDB type holding the same values.
    pub fn duckdb_name(self) -> String {
        match self {
            Self::Boolean => "BOOLEAN".to_owned(),
            Self::TinyInt => "TINYINT".to_owned(),
            Self::SmallInt => "SMALLINT".to_owned(),
            Self::Int => "INTEGER".to_owned(),
            Self::BigInt => "BIGINT".to_owned(),
            Self::Float => "FLOAT".to_owned(),
            Self::Double => "DOUBLE".to_owned(),
            Self::Decimal { precision, scale } => format!("DECIMAL({precision}, {scale})"),
            Self::String => "VARCHAR".to_owned(),
            Self::Date => "DATE".to_owned(),
            Self::Timestamp => "TIMESTAMPTZ".to_owned(),
            Self::TimestampNtz => "TIMESTAMP".to_owned(),
            Self::Null => "\"NULL\"".to_owned(),
        }
    }

    /// Whether the type is an exact integer.
    pub fn is_integral(self) -> bool {
        self.integer_digits().is_some()
    }

    /// The Databricks name of the type, as a result's metadata spells it.
    pub fn name(self) -> String {
        match self {
            Self::Boolean => "BOOLEAN".to_owned(),
            Self::TinyInt => "TINYINT".to_owned(),
            Self::SmallInt => "SMALLINT".to_owned(),
            Self::Int => "INT".to_owned(),
            Self::BigInt => "BIGINT".to_owned(),
            Self::Float => "FLOAT".to_owned(),
            Self::Double => "DOUBLE".to_owned(),
            Self::Decimal { precision, scale } => format!("DECIMAL({precision},{scale})"),
            Self::String => "STRING".to_owned(),
            Self::Date => "DATE".to_owned(),
            Self::Timestamp => "TIMESTAMP".to_owned(),
            Self::TimestampNtz => "TIMESTAMP_NTZ".to_owned(),
            Self::Null => "VOID".to_owned(),
        }
    }

    /// The decimal digits an integer type's values may have, `None` for other types.
    pub fn integer_digits(self) -> Option<u8> {
        match self {
            Self::TinyInt => Some(3),
            Self::SmallInt => Some(5),
            Self::Int => Some(10),
            Self::BigInt => Some(19),
            _ => None,
        }
    }

    /// Whether the type is a number: an integer, a float or a decimal.
    pub fn is_numeric(self) -> bool {
        self.is_integral() || matches!(self, Self::Float | Self::Double | Self::Decimal { .. })
    }
}

/// DuckDB SQL for one Databricks statement, and what the conversion relied on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Converted {
    /// The DuckDB SQL.
    pub sql: String,
    /// Each result column's name and Databricks type.
    pub columns: Vec<(String, DatabricksType)>,
    /// The rules applied.
    pub provenance: Provenance,
}

/// One converted expression: its DuckDB SQL and its Databricks type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Emitted {
    /// The DuckDB SQL, parenthesised wherever precedence could change its reading.
    pub(crate) sql: String,
    /// The Databricks type.
    pub(crate) data_type: DatabricksType,
    /// The literal the expression is, when it is one.
    pub(crate) literal: Option<Literal>,
}

/// A literal as written: a number, with its sign, or a string's value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Literal {
    /// A number such as `-7`, `2.50` or `1e3`.
    Number(String),
    /// A string's value.
    Text(String),
}

/// What a function's arguments may be and what it returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signature {
    /// One string in, one string out.
    StringToString,
    /// One number in, the same type out.
    SameNumber,
    /// Two or more values of one type in, that type out.
    SameType,
    /// Exactly two values of one type in, that type out.
    TwoOfOneType,
    /// One string in, spaces trimmed: Databricks trims only U+0020, so DuckDB is told the same.
    TrimSpaces,
}
