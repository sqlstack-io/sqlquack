//! The implemented dialect profiles.

use crate::dialects::models::{DuckDb, Profiles, Snowflake, SnowflakeQuotedIgnoringCase};

/// The Snowflake profile.
pub const SNOWFLAKE: Snowflake = Snowflake;

/// The Snowflake profile under QUOTED_IDENTIFIERS_IGNORE_CASE.
pub const SNOWFLAKE_QUOTED_IGNORING_CASE: SnowflakeQuotedIgnoringCase = SnowflakeQuotedIgnoringCase;

/// The DuckDB profile.
pub const DUCKDB: DuckDb = DuckDb;

/// Snowflake SQL resolved so its names stay apart in DuckDB, as conversion needs.
pub const SNOWFLAKE_TO_DUCKDB: Profiles = Profiles {
    source: &SNOWFLAKE,
    target: &DUCKDB,
};
