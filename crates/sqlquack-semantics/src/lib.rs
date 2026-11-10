//! SQL semantics for SQLQuack: scopes, name resolution, types and lineage, per dialect.

#![forbid(unsafe_code)]

#[cfg(feature = "bigquery")]
pub mod bigquery;
#[cfg(feature = "clickhouse")]
pub mod clickhouse;
#[cfg(feature = "databricks")]
pub mod databricks;
pub mod dialects;
#[cfg(feature = "duckdb")]
pub mod duckdb;
pub mod lineage;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod schemas;
pub mod scopes;
#[cfg(feature = "snowflake")]
pub mod snowflake;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "sqlserver")]
pub mod sqlserver;

pub use crate::lineage::main::query_lineage::query_lineage;
pub use crate::lineage::main::query_lineage_and_findings::query_lineage_and_findings;
pub use crate::lineage::main::query_name_findings::query_name_findings;
pub use crate::lineage::main::statement_lineage::statement_lineage;
pub use crate::lineage::main::statement_lineage_and_findings::statement_lineage_and_findings;
pub use crate::lineage::main::statement_name_findings::statement_name_findings;
