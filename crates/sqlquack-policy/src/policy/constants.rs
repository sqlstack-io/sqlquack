//! Policy file identity.

/// The policy file at the repository root.
pub const POLICY_FILE_NAME: &str = "sqlquack.toml";

/// The time zone every session profile runs in, Snowflake's `TIMEZONE` and DuckDB's `TimeZone`.
pub const SESSION_TIME_ZONE: &str = "UTC";

/// The Snowflake session parameter naming the session time zone.
pub const TIMEZONE_PARAMETER: &str = "TIMEZONE";

/// The only schema version this build understands.
pub const SCHEMA_VERSION: u32 = 1;
