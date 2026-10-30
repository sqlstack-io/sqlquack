//! The implemented dialect profiles.

use crate::dialects::models::{DuckDb, Profiles, Snowflake};

/// The Snowflake profile.
pub const SNOWFLAKE: Snowflake = Snowflake;

/// The DuckDB profile.
pub const DUCKDB: DuckDb = DuckDb;

/// Snowflake SQL resolved so its names stay apart in DuckDB, as conversion needs.
pub const SNOWFLAKE_TO_DUCKDB: Profiles = Profiles {
    source: &SNOWFLAKE,
    target: &DUCKDB,
};
