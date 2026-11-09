//! SQL layout that changes only token spacing and keyword case, ported from SQLClean's formatter.

#![forbid(unsafe_code)]

#[cfg(feature = "bigquery")]
pub(crate) mod bigquery;
#[cfg(feature = "databricks")]
pub(crate) mod databricks;
pub(crate) mod dialects;
#[cfg(feature = "duckdb")]
pub(crate) mod duckdb;
pub mod format;
#[cfg(feature = "postgres")]
pub(crate) mod postgres;
#[cfg(feature = "snowflake")]
pub(crate) mod snowflake;
#[cfg(feature = "sqlserver")]
pub(crate) mod sqlserver;

pub use crate::format::main::format_sql::format_sql;
pub use crate::format::models::{FormatDialect, FormatOptions, FormatRefusal, Formatted};
