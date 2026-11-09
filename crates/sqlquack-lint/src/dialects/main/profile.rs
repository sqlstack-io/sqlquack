//! `profile`: the registry of each dialect's lint profile.

#[cfg(feature = "bigquery")]
use crate::bigquery::profile::main::bigquery_profile::bigquery_profile;
#[cfg(feature = "databricks")]
use crate::databricks::profile::main::databricks_profile::databricks_profile;
use crate::dialects::models::LintProfile;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::main::duckdb_profile::duckdb_profile;
use crate::lint::models::LintDialect;
#[cfg(feature = "postgres")]
use crate::postgres::profile::main::postgres_profile::postgres_profile;
#[cfg(feature = "snowflake")]
use crate::snowflake::profile::main::snowflake_profile::snowflake_profile;
#[cfg(feature = "sqlite")]
use crate::sqlite::profile::main::sqlite_profile::sqlite_profile;

/// The lint profile of `dialect`; one built without its feature reads nothing, though no name reaches it.
pub(crate) fn profile(dialect: LintDialect) -> LintProfile {
    match dialect {
        #[cfg(feature = "snowflake")]
        LintDialect::Snowflake => snowflake_profile(),
        #[cfg(not(feature = "snowflake"))]
        LintDialect::Snowflake => crate::dialects::_helpers::unbuilt_profile::unbuilt_profile(),
        #[cfg(feature = "duckdb")]
        LintDialect::DuckDB => duckdb_profile(),
        #[cfg(not(feature = "duckdb"))]
        LintDialect::DuckDB => crate::dialects::_helpers::unbuilt_profile::unbuilt_profile(),
        #[cfg(feature = "postgres")]
        LintDialect::PostgreSQL => postgres_profile(),
        #[cfg(not(feature = "postgres"))]
        LintDialect::PostgreSQL => crate::dialects::_helpers::unbuilt_profile::unbuilt_profile(),
        #[cfg(feature = "sqlite")]
        LintDialect::SQLite => sqlite_profile(),
        #[cfg(not(feature = "sqlite"))]
        LintDialect::SQLite => crate::dialects::_helpers::unbuilt_profile::unbuilt_profile(),
        #[cfg(feature = "bigquery")]
        LintDialect::BigQuery => bigquery_profile(),
        #[cfg(not(feature = "bigquery"))]
        LintDialect::BigQuery => crate::dialects::_helpers::unbuilt_profile::unbuilt_profile(),
        #[cfg(feature = "databricks")]
        LintDialect::Databricks => databricks_profile(),
        #[cfg(not(feature = "databricks"))]
        LintDialect::Databricks => crate::dialects::_helpers::unbuilt_profile::unbuilt_profile(),
    }
}
