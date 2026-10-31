//! Policy file identity.

/// The policy file at the repository root.
pub const POLICY_FILE_NAME: &str = "sqlquack.toml";

/// The time zone every session profile runs in, Snowflake's `TIMEZONE` and DuckDB's `TimeZone`.
pub const SESSION_TIME_ZONE: &str = "UTC";

/// The Snowflake session parameter naming the session time zone.
pub const TIMEZONE_PARAMETER: &str = "TIMEZONE";

/// The Snowflake session parameter making quoted identifiers resolve to upper case too.
pub const QUOTED_IDENTIFIERS_IGNORE_CASE: &str = "QUOTED_IDENTIFIERS_IGNORE_CASE";

/// Session parameters that cannot change a result, accepted and ignored, each with why.
pub const RESULT_NEUTRAL_SESSION_PARAMETERS: &[(&str, &str)] = &[
    ("QUERY_TAG", "a label recorded in query history"),
    (
        "USE_CACHED_RESULT",
        "the result cache returns only a result identical to recomputing it",
    ),
    (
        "STATEMENT_TIMEOUT_IN_SECONDS",
        "a timeout can only cancel a statement, never change its rows",
    ),
    (
        "STATEMENT_QUEUED_TIMEOUT_IN_SECONDS",
        "a queueing timeout can only cancel a statement, never change its rows",
    ),
    (
        "ABORT_DETACHED_QUERY",
        "whether a statement is cancelled when its client disconnects",
    ),
    (
        "CLIENT_SESSION_KEEP_ALIVE",
        "a client heartbeat keeping the session open",
    ),
];

/// The profile holding Snowflake's documented defaults, which a parameter left unset takes.
pub const DEFAULT_SESSION_PROFILE: &str = "utc-v1";

/// The only schema version this build understands.
pub const SCHEMA_VERSION: u32 = 1;
