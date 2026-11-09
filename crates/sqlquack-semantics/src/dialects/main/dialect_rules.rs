//! `dialect_rules`: the lineage rules of a profile's dialect.

#[cfg(feature = "bigquery")]
use crate::bigquery::profile::main::bigquery_rules::bigquery_rules;
#[cfg(feature = "clickhouse")]
use crate::clickhouse::profile::main::clickhouse_rules::clickhouse_rules;
#[cfg(feature = "databricks")]
use crate::databricks::profile::main::databricks_rules::databricks_rules;
use sqlquack_syntax::Dialect;

use crate::dialects::types::DialectProfile;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::main::duckdb_rules::duckdb_rules;
use crate::lineage::models::SharedRules;
use crate::lineage::types::DialectRules;
#[cfg(feature = "postgres")]
use crate::postgres::profile::main::postgres_rules::postgres_rules;
#[cfg(feature = "snowflake")]
use crate::snowflake::profile::main::snowflake_rules::snowflake_rules;
#[cfg(feature = "sqlite")]
use crate::sqlite::profile::main::sqlite_rules::sqlite_rules;
#[cfg(feature = "sqlserver")]
use crate::sqlserver::profile::main::sqlserver_rules::sqlserver_rules;

/// How lineage types values in `profile`'s dialect; the shared rules alone for one not built in.
pub(crate) fn dialect_rules(profile: &dyn DialectProfile) -> &'static dyn DialectRules {
    let built: Option<&'static dyn DialectRules> = match profile.dialect() {
        #[cfg(feature = "snowflake")]
        Dialect::Snowflake => Some(snowflake_rules()),
        #[cfg(not(feature = "snowflake"))]
        Dialect::Snowflake => None,
        #[cfg(feature = "duckdb")]
        Dialect::DuckDB => Some(duckdb_rules()),
        #[cfg(not(feature = "duckdb"))]
        Dialect::DuckDB => None,
        #[cfg(feature = "postgres")]
        Dialect::PostgreSQL => Some(postgres_rules()),
        #[cfg(not(feature = "postgres"))]
        Dialect::PostgreSQL => None,
        #[cfg(feature = "sqlite")]
        Dialect::SQLite => Some(sqlite_rules()),
        #[cfg(not(feature = "sqlite"))]
        Dialect::SQLite => None,
        #[cfg(feature = "bigquery")]
        Dialect::BigQuery => Some(bigquery_rules()),
        #[cfg(not(feature = "bigquery"))]
        Dialect::BigQuery => None,
        #[cfg(feature = "databricks")]
        Dialect::Databricks => Some(databricks_rules()),
        #[cfg(not(feature = "databricks"))]
        Dialect::Databricks => None,
        #[cfg(feature = "clickhouse")]
        Dialect::ClickHouse => Some(clickhouse_rules()),
        #[cfg(not(feature = "clickhouse"))]
        Dialect::ClickHouse => None,
        #[cfg(feature = "sqlserver")]
        Dialect::SqlServer => Some(sqlserver_rules()),
        #[cfg(not(feature = "sqlserver"))]
        Dialect::SqlServer => None,
    };
    built.unwrap_or(&SharedRules)
}
