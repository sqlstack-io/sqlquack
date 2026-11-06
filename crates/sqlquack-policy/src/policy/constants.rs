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

/// The fields every `[divergences.<id>]` entry must give.
pub const DIVERGENCE_FIELDS: [&str; 5] = ["constructs", "default", "strict", "reason", "failure"];

/// The most character edits between a misspelt divergence id and the one it suggests.
pub const MAX_SPELLING_DISTANCE: usize = 2;

/// HASH values differ from Snowflake's; equal inputs hash equally within one run.
pub const HASH_DIVERGENCE: &str = "hash";
/// The divergence where DuckDB raises for a nanosecond timestamp outside its range.
pub const TIMESTAMP_RANGE: &str = "timestamp_range";
/// PERCENTILE_CONT raises where it would interpolate between two different values.
pub const PERCENTILE_INTERPOLATION: &str = "percentile_interpolation";
/// TRY_ conversions of text to DATE raise for spellings no recording shows.
pub const DATE_TEXT_SPELLING: &str = "date_text_spelling";
/// The divergence of the current date and time, which differ between engines and runs.
pub const CURRENT_TIME: &str = "current_time";
/// The divergence of a timestamp's text where truncating and rounding to milliseconds differ.
pub const TIMESTAMP_TEXT_ROUNDING: &str = "timestamp_text_rounding";
/// The divergence of ANY_VALUE, whose choice among a group's values differs between engines.
pub const ANY_VALUE_CHOICE: &str = "any_value_choice";
/// The divergence of MODE, whose choice among values tied for most frequent differs between engines.
pub const MODE_TIES: &str = "mode_ties";
/// The divergence where DuckDB raises for a zoned timestamp with a sub-microsecond part.
pub const TIMESTAMP_TZ_SUBMICRO: &str = "timestamp_tz_submicro";
/// MODE over no rows without GROUP BY, which Snowflake answers by plan: raised.
pub const MODE_OF_NO_ROWS: &str = "mode_of_no_rows";
/// The assumption that a TIMESTAMP_TZ source column holds the offset its schema declares.
pub const DECLARED_OFFSET: &str = "declared_offset";
/// The assumption that the session time zone is UTC, in Snowflake and DuckDB alike.
pub const SESSION_TIMEZONE_UTC: &str = "session_timezone_utc";
/// The divergence where DuckDB raises an error for a value beyond its numeric range.
pub const NUMERIC_RANGE: &str = "numeric_range";
/// The divergence for ARRAY_AGG without WITHIN GROUP of several distinct values.
pub const ARRAY_AGG_ORDER: &str = "array_agg_order";
/// The divergence for LISTAGG of different values tied on every WITHIN GROUP key.
pub const LISTAGG_ORDER: &str = "listagg_order";
/// The assumption a Python project function call relies on: its handler runs locally as in Snowflake.
pub const PROJECT_UDF: &str = "project_udf";
/// The divergence of a Python value whose crossing no recording shows, which SQLQuack's runtime raises.
pub const PYTHON_VALUE_CROSSING: &str = "python_value_crossing";
/// The assumption a user override relies on: its DuckDB SQL gives what the function it replaces gives.
pub const USER_OVERRIDE: &str = "user_override";
/// A zone name per row whose spelling no recording shows Snowflake accepting, raised.
pub const TIME_ZONE_SPELLING: &str = "time_zone_spelling";
/// The divergence of zoned text whose offset is not +00:00, raised where its wall time matters.
pub const TIMESTAMP_TZ_OFFSET: &str = "timestamp_tz_offset";
