//! The implemented dialect profiles, each built with its dialect's feature.

#[cfg(feature = "bigquery")]
use crate::bigquery::profile::models::BigQuery;
#[cfg(feature = "clickhouse")]
use crate::clickhouse::profile::models::ClickHouse;
#[cfg(feature = "databricks")]
use crate::databricks::profile::models::Databricks;
#[cfg(all(feature = "snowflake", feature = "duckdb"))]
use crate::dialects::models::Profiles;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::models::DuckDb;
#[cfg(feature = "postgres")]
use crate::postgres::profile::models::Postgres;
#[cfg(feature = "snowflake")]
use crate::snowflake::profile::models::{Snowflake, SnowflakeQuotedIgnoringCase};
#[cfg(feature = "sqlite")]
use crate::sqlite::profile::models::Sqlite;
#[cfg(feature = "sqlserver")]
use crate::sqlserver::profile::models::SqlServer;

/// The Snowflake profile.
#[cfg(feature = "snowflake")]
pub const SNOWFLAKE: Snowflake = Snowflake;

/// The Snowflake profile under QUOTED_IDENTIFIERS_IGNORE_CASE.
#[cfg(feature = "snowflake")]
pub const SNOWFLAKE_QUOTED_IGNORING_CASE: SnowflakeQuotedIgnoringCase = SnowflakeQuotedIgnoringCase;

/// The DuckDB profile.
#[cfg(feature = "duckdb")]
pub const DUCKDB: DuckDb = DuckDb;

/// The PostgreSQL profile.
#[cfg(feature = "postgres")]
pub const POSTGRES: Postgres = Postgres;

/// The ClickHouse profile.
#[cfg(feature = "clickhouse")]
pub const CLICKHOUSE: ClickHouse = ClickHouse;

/// The BigQuery profile.
#[cfg(feature = "bigquery")]
pub const BIGQUERY: BigQuery = BigQuery;

/// The SQLite profile.
#[cfg(feature = "sqlite")]
pub const SQLITE: Sqlite = Sqlite;

/// The SQL Server profile, under the server's default case-insensitive collation.
#[cfg(feature = "sqlserver")]
pub const SQL_SERVER: SqlServer = SqlServer;

/// The Databricks SQL profile.
#[cfg(feature = "databricks")]
pub const DATABRICKS: Databricks = Databricks;

/// Snowflake SQL resolved so its names stay apart in DuckDB, as conversion needs.
#[cfg(all(feature = "snowflake", feature = "duckdb"))]
pub const SNOWFLAKE_TO_DUCKDB: Profiles = Profiles {
    source: &SNOWFLAKE,
    target: &DUCKDB,
};
