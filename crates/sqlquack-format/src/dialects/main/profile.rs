//! `profile`: the registry of each dialect's format profile.

#[cfg(feature = "bigquery")]
use crate::bigquery::profile::main::bigquery_profile::bigquery_profile;
#[cfg(feature = "databricks")]
use crate::databricks::profile::main::databricks_profile::databricks_profile;
use crate::dialects::models::FormatProfile;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::main::duckdb_profile::duckdb_profile;
use crate::format::models::FormatDialect;
#[cfg(feature = "postgres")]
use crate::postgres::profile::main::postgres_profile::postgres_profile;
#[cfg(feature = "snowflake")]
use crate::snowflake::profile::main::snowflake_profile::snowflake_profile;
#[cfg(feature = "sqlserver")]
use crate::sqlserver::profile::main::sqlserver_profile::sqlserver_profile;

/// The format profile of `dialect`, or why the formatter cannot lay out its SQL.
pub(crate) fn profile(dialect: FormatDialect) -> Result<FormatProfile, &'static str> {
    match dialect {
        #[cfg(feature = "snowflake")]
        FormatDialect::Snowflake => Ok(snowflake_profile()),
        #[cfg(not(feature = "snowflake"))]
        FormatDialect::Snowflake => Err(crate::format::constants::NOT_COMPILED_FAILURE),
        #[cfg(feature = "duckdb")]
        FormatDialect::DuckDB => Ok(duckdb_profile()),
        #[cfg(not(feature = "duckdb"))]
        FormatDialect::DuckDB => Err(crate::format::constants::NOT_COMPILED_FAILURE),
        #[cfg(feature = "postgres")]
        FormatDialect::PostgreSQL => Ok(postgres_profile()),
        #[cfg(not(feature = "postgres"))]
        FormatDialect::PostgreSQL => Err(crate::format::constants::NOT_COMPILED_FAILURE),
        #[cfg(feature = "bigquery")]
        FormatDialect::BigQuery => Ok(bigquery_profile()),
        #[cfg(not(feature = "bigquery"))]
        FormatDialect::BigQuery => Err(crate::format::constants::NOT_COMPILED_FAILURE),
        #[cfg(feature = "sqlserver")]
        FormatDialect::SqlServer => Ok(sqlserver_profile()),
        #[cfg(not(feature = "sqlserver"))]
        FormatDialect::SqlServer => Err(crate::format::constants::NOT_COMPILED_FAILURE),
        #[cfg(feature = "databricks")]
        FormatDialect::Databricks => databricks_profile(),
        #[cfg(not(feature = "databricks"))]
        FormatDialect::Databricks => Err(crate::format::constants::NOT_COMPILED_FAILURE),
    }
}
