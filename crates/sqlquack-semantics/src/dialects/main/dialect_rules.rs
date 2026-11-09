//! `dialect_rules`: the lineage rules of a profile's dialect.

#[cfg(feature = "bigquery")]
use crate::bigquery::profile::main::bigquery_rules::bigquery_rules;
#[cfg(feature = "clickhouse")]
use crate::clickhouse::profile::main::clickhouse_rules::clickhouse_rules;
#[cfg(feature = "databricks")]
use crate::databricks::profile::main::databricks_rules::databricks_rules;
use crate::dialects::types::DialectProfile;
#[cfg(feature = "duckdb")]
use crate::duckdb::profile::main::duckdb_rules::duckdb_rules;
#[cfg(feature = "bigquery")]
use crate::lineage::constants::BIGQUERY_PROFILE;
#[cfg(feature = "clickhouse")]
use crate::lineage::constants::CLICKHOUSE_PROFILE;
#[cfg(feature = "databricks")]
use crate::lineage::constants::DATABRICKS_PROFILE;
#[cfg(feature = "duckdb")]
use crate::lineage::constants::DUCKDB_PROFILE;
#[cfg(feature = "postgres")]
use crate::lineage::constants::POSTGRES_PROFILE;
#[cfg(feature = "snowflake")]
use crate::lineage::constants::SNOWFLAKE_PROFILE;
#[cfg(feature = "sqlite")]
use crate::lineage::constants::SQLITE_PROFILE;
#[cfg(feature = "sqlserver")]
use crate::lineage::constants::SQLSERVER_PROFILE;
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

/// How lineage types values in `profile`'s dialect; the shared rules alone for any other.
pub(crate) fn dialect_rules(profile: &dyn DialectProfile) -> &'static dyn DialectRules {
    match profile.name() {
        #[cfg(feature = "snowflake")]
        SNOWFLAKE_PROFILE => snowflake_rules(),
        #[cfg(feature = "duckdb")]
        DUCKDB_PROFILE => duckdb_rules(),
        #[cfg(feature = "postgres")]
        POSTGRES_PROFILE => postgres_rules(),
        #[cfg(feature = "sqlite")]
        SQLITE_PROFILE => sqlite_rules(),
        #[cfg(feature = "bigquery")]
        BIGQUERY_PROFILE => bigquery_rules(),
        #[cfg(feature = "databricks")]
        DATABRICKS_PROFILE => databricks_rules(),
        #[cfg(feature = "clickhouse")]
        CLICKHOUSE_PROFILE => clickhouse_rules(),
        #[cfg(feature = "sqlserver")]
        SQLSERVER_PROFILE => sqlserver_rules(),
        _ => &SharedRules,
    }
}
