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
    time in the session's time zone, and DATEADD, DATEDIFF, TIMESTAMPDIFF, TIMEDIFF, COALESCE and \
    comparisons read a TIMESTAMP_LTZ, or a TIMESTAMP_TZ compared with text that has no offset, in \
    it too, so they agree only where Snowflake's TIMEZONE and DuckDB's TimeZone are both UTC";

/// Where the built-in default lets 38-digit arithmetic rely on DuckDB raising at the limit.
pub const BUILTIN_NUMERIC_CONSTRUCTS: [&str; 4] = ["node:add", "node:sub", "node:mul", "fn:SUM"];

/// The edge case of numeric_range under the built-in default, printed with the warning.
pub const BUILTIN_NUMERIC_REASON: &str = "if a sum, difference, product or SUM needs more than 38 \
    digits, DuckDB raises an Overflow error where Snowflake can still return the number; it never \
    returns a different number. --strict refuses this";

/// Where the built-in default lets date arithmetic rely on the converted SQL raising out of range.
pub const BUILTIN_TIMESTAMP_CONSTRUCTS: [&str; 11] = [
    "fn:DATEADD",
    "fn:TIMEADD",
    "fn:TIMESTAMPADD",
    "node:add",
    "node:sub",
    "node:eq",
    "node:neq",
    "node:lt",
    "node:lte",
    "node:gt",
    "node:gte",
];

/// The edge case of timestamp_range under the built-in default, printed with the warning.
pub const BUILTIN_TIMESTAMP_REASON: &str = "if date or time arithmetic, or a timestamp \
    compared at nanosecond precision, lands outside 1677-09-21 00:12:43.145224192 to \
    2262-04-11 23:47:16.854775807 UTC, the converted SQL raises an error where Snowflake returns \
    or compares the timestamp; it never returns a different value. --strict refuses this";

/// Where the built-in default reads a TIMESTAMP_LTZ, such as the clock, in the UTC session.
pub const BUILTIN_ZONE_CONSTRUCTS: [&str; 18] = [
    "fn:CURRENT_DATE",
    "fn:CURRENT_TIMESTAMP",
    "fn:DATEADD",
    "fn:DATEDIFF",
    "node:date_diff",
    "node:eq",
    "node:neq",
    "node:lt",
    "node:lte",
    "node:gt",
    "node:gte",
    "fn:TIMESTAMPDIFF",
    "fn:TIMEDIFF",
    "fn:COALESCE",
    "fn:IFNULL",
    "fn:NVL",
    "node:coalesce",
    "node:nvl",
];

/// Where the built-in default lets reading a VARIANT rely on the converted SQL's JSON guards.
pub const BUILTIN_JSON_CONSTRUCTS: [&str; 4] = ["node:column", "node:cast", "fn:GET", "fn:FLATTEN"];

/// The edge case of json_fidelity under the built-in default, printed with the warning.
pub const BUILTIN_JSON_REASON: &str = "if a VARIANT holds JSON DuckDB might read differently \
    from Snowflake, such as an exponent, -0.0, a decimal longer than a double keeps, a repeated \
    key or text that is not strict JSON, or a value a cast cannot match, the converted SQL raises \
    an error for the whole query; it never returns a different value. --strict refuses this";

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
