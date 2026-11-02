//! The implemented dialect profiles.

use crate::dialects::models::{
    DuckDb, Postgres, Profiles, Snowflake, SnowflakeQuotedIgnoringCase, Sqlite,
};

/// The Snowflake profile.
pub const SNOWFLAKE: Snowflake = Snowflake;

/// The Snowflake profile under QUOTED_IDENTIFIERS_IGNORE_CASE.
pub const SNOWFLAKE_QUOTED_IGNORING_CASE: SnowflakeQuotedIgnoringCase = SnowflakeQuotedIgnoringCase;

/// The DuckDB profile.
pub const DUCKDB: DuckDb = DuckDb;

/// The PostgreSQL profile.
pub const POSTGRES: Postgres = Postgres;

/// The SQLite profile.
pub const SQLITE: Sqlite = Sqlite;

/// Snowflake SQL resolved so its names stay apart in DuckDB, as conversion needs.
pub const SNOWFLAKE_TO_DUCKDB: Profiles = Profiles {
    source: &SNOWFLAKE,
    target: &DUCKDB,
};
/// The length of Snowflake text declared without one.
pub(crate) const SNOWFLAKE_LONGEST_VARCHAR: u32 = 16_777_216;
/// What Snowflake's TIMESTAMP is under the default TIMESTAMP_TYPE_MAPPING.
pub(crate) const SNOWFLAKE_TIMESTAMP_TYPE: &str = "TIMESTAMP_NTZ";
