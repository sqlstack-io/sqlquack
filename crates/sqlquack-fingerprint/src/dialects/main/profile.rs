//! `profile`: the registry of each dialect's fingerprint profile.

#[cfg(feature = "bigquery")]
use crate::bigquery::profile::main::bigquery_profile::bigquery_profile;
#[cfg(feature = "databricks")]
use crate::databricks::profile::main::databricks_profile::databricks_profile;
use crate::dialects::models::FingerprintProfile;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::main::duckdb_profile::duckdb_profile;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::main::generic_profile::generic_profile;
use crate::fingerprint::models::FingerprintDialect;
#[cfg(feature = "postgres")]
use crate::postgres::profile::main::postgres_profile::postgres_profile;
#[cfg(feature = "snowflake")]
use crate::snowflake::profile::main::snowflake_profile::snowflake_profile;
#[cfg(feature = "sqlite")]
use crate::sqlite::profile::main::sqlite_profile::sqlite_profile;
#[cfg(feature = "sqlserver")]
use crate::sqlserver::profile::main::sqlserver_profile::sqlserver_profile;

/// The fingerprint profile of `dialect`, if the crate was built with its feature; SQL in no named dialect reads DuckDB's tokens.
pub(crate) fn profile(dialect: FingerprintDialect) -> Option<FingerprintProfile> {
    match dialect {
        #[cfg(feature = "duckdb")]
        FingerprintDialect::Generic => Some(generic_profile()),
        #[cfg(not(feature = "duckdb"))]
        FingerprintDialect::Generic => None,
        #[cfg(feature = "bigquery")]
        FingerprintDialect::BigQuery => Some(bigquery_profile()),
        #[cfg(not(feature = "bigquery"))]
        FingerprintDialect::BigQuery => None,
        #[cfg(feature = "databricks")]
        FingerprintDialect::Databricks => Some(databricks_profile()),
        #[cfg(not(feature = "databricks"))]
        FingerprintDialect::Databricks => None,
        #[cfg(feature = "duckdb")]
        FingerprintDialect::DuckDB => Some(duckdb_profile()),
        #[cfg(not(feature = "duckdb"))]
        FingerprintDialect::DuckDB => None,
        #[cfg(feature = "postgres")]
        FingerprintDialect::PostgreSQL => Some(postgres_profile()),
        #[cfg(not(feature = "postgres"))]
        FingerprintDialect::PostgreSQL => None,
        #[cfg(feature = "snowflake")]
        FingerprintDialect::Snowflake => Some(snowflake_profile()),
        #[cfg(not(feature = "snowflake"))]
        FingerprintDialect::Snowflake => None,
        #[cfg(feature = "sqlite")]
        FingerprintDialect::SQLite => Some(sqlite_profile()),
        #[cfg(not(feature = "sqlite"))]
        FingerprintDialect::SQLite => None,
        #[cfg(feature = "sqlserver")]
        FingerprintDialect::SqlServer => Some(sqlserver_profile()),
        #[cfg(not(feature = "sqlserver"))]
        FingerprintDialect::SqlServer => None,
    }
}
