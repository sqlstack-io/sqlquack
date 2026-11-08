//! The dialects a fingerprint reads SQL in.

use crate::fingerprint::errors::FingerprintError;

/// A dialect SQLQuack tokenizes for fingerprints; `Generic` reads DuckDB's tokens with a smaller keyword list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FingerprintDialect {
    /// No named dialect.
    Generic,
    /// DuckDB.
    DuckDB,
    /// PostgreSQL, through its native scanner.
    PostgreSQL,
    /// Snowflake.
    Snowflake,
    /// SQLite.
    SQLite,
}

impl FingerprintDialect {
    /// The dialect named `name`: `generic`, `duckdb`, `postgres`, `postgresql`, `snowflake` or `sqlite`.
    pub fn from_name(name: &str) -> Result<Self, FingerprintError> {
        match name {
            "generic" => Ok(Self::Generic),
            "duckdb" => Ok(Self::DuckDB),
            "postgres" | "postgresql" => Ok(Self::PostgreSQL),
            "snowflake" => Ok(Self::Snowflake),
            "sqlite" => Ok(Self::SQLite),
            _ => Err(FingerprintError::UnknownDialect(name.to_owned())),
        }
    }
}
