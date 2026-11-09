//! The dialects a fingerprint reads SQL in.

use crate::fingerprint::errors::FingerprintError;

/// A dialect SQLQuack tokenizes for fingerprints; `Generic` reads DuckDB's tokens with a smaller keyword list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FingerprintDialect {
    /// No named dialect.
    Generic,
    /// BigQuery, through its native scanner.
    BigQuery,
    /// Databricks: the generic profile's tokens and keywords, except the keywords it lets name columns and tables.
    Databricks,
    /// DuckDB.
    DuckDB,
    /// PostgreSQL, through its native scanner.
    PostgreSQL,
    /// Snowflake.
    Snowflake,
    /// SQLite.
    SQLite,
    /// SQL Server, through its native scanner.
    SqlServer,
}

impl FingerprintDialect {
    /// The dialect named `name`: `generic`, `bigquery`, `databricks`, `duckdb`, `postgres`, `postgresql`, `snowflake`, `sqlite`, `sqlserver`, `tsql` or `mssql`.
    pub fn from_name(name: &str) -> Result<Self, FingerprintError> {
        match name {
            "generic" => Ok(Self::Generic),
            "bigquery" => Ok(Self::BigQuery),
            "databricks" => Ok(Self::Databricks),
            "duckdb" => Ok(Self::DuckDB),
            "postgres" | "postgresql" => Ok(Self::PostgreSQL),
            "snowflake" => Ok(Self::Snowflake),
            "sqlite" => Ok(Self::SQLite),
            "sqlserver" | "tsql" | "mssql" => Ok(Self::SqlServer),
            _ => Err(FingerprintError::UnknownDialect(name.to_owned())),
        }
    }
}
