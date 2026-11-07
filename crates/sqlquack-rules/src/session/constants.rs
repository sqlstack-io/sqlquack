//! Session constants.

/// The profile name reported when the built-in default stands in for a policy.
pub const BUILTIN_PROFILE: &str = "the built-in default";

/// The clock functions the built-in default converts with a warning outside strict mode.
pub const BUILTIN_CLOCK_CONSTRUCTS: [&str; 2] = ["fn:CURRENT_DATE", "fn:CURRENT_TIMESTAMP"];

/// Why the built-in default converts the clock functions, printed with the warning.
pub const BUILTIN_CLOCK_REASON: &str = "CURRENT_DATE() and CURRENT_TIMESTAMP() take their value \
    when the query runs, in DuckDB as in Snowflake, so the value differs between runs; the \
    converted SQL reads DuckDB's clock in the DuckDB session's time zone, which must match \
    Snowflake's TIMEZONE (UTC) for the date and time to agree. --strict refuses them";

/// The session time zone the clock functions rest on under the built-in default.
pub const BUILTIN_CLOCK_ZONE: &str = "CURRENT_DATE() and CURRENT_TIMESTAMP() give the date and \
    time in the session's time zone, so they agree only where Snowflake's TIMEZONE and DuckDB's \
    TimeZone are both UTC";

/// Where the built-in default reads a TIMESTAMP_LTZ, such as the clock, in the UTC session.
pub const BUILTIN_ZONE_CONSTRUCTS: [&str; 11] = [
    "fn:CURRENT_DATE",
    "fn:CURRENT_TIMESTAMP",
    "fn:DATEDIFF",
    "node:date_diff",
    "fn:TIMESTAMPDIFF",
    "fn:TIMEDIFF",
    "fn:COALESCE",
    "fn:IFNULL",
    "fn:NVL",
    "node:coalesce",
    "node:nvl",
];

/// Where the built-in default lets a naive timestamp joined with a local one raise on nanoseconds.
pub const BUILTIN_SUBMICRO_CONSTRUCTS: [&str; 5] = [
    "fn:COALESCE",
    "fn:IFNULL",
    "fn:NVL",
    "node:coalesce",
    "node:nvl",
];

/// The edge case of timestamp_tz_submicro under the built-in default, printed with the warning.
pub const BUILTIN_SUBMICRO_REASON: &str = "a TIMESTAMP_NTZ joined with a TIMESTAMP_LTZ, as in \
    COALESCE(shipped_at, CURRENT_TIMESTAMP()), becomes a DuckDB TIMESTAMPTZ, which holds \
    microseconds; a naive value with digits below the microsecond raises an error instead of \
    being rounded; it never returns a different value. --strict refuses this";
