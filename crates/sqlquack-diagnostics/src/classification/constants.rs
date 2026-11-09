//! Reserved names shared with callers that rewrite project-specific calls before parsing.

/// The function name callers substitute for project user-defined functions.
pub const PROJECT_FUNCTION_PLACEHOLDER: &str = "__sqlquack_udf";

/// The construct name of a project user-defined function call.
pub const PROJECT_FUNCTION_CONSTRUCT: &str = "<udf>";
/// The name callers substitute for a microbatch cursor bound they cannot render.
pub const CURSOR_PLACEHOLDER: &str = "__sqlquack_cursor";
/// The construct name of an unrendered microbatch cursor bound.
pub const CURSOR_CONSTRUCT: &str = "<cursor>";
/// Spellings of a timestamp without time zone a cast is classified under, the first one used.
pub const NO_TIME_ZONE_ALIASES: [&str; 2] = ["TIMESTAMPNTZ", "TIMESTAMP_NTZ"];
/// Spellings of a timestamp with local time zone, the first one used.
pub const LOCAL_TIME_ZONE_ALIASES: [&str; 2] = ["TIMESTAMPLTZ", "TIMESTAMP_LTZ"];
/// Spellings of a timestamp with time zone, the first one used.
pub const TIME_ZONE_ALIASES: [&str; 2] = ["TIMESTAMPTZ", "TIMESTAMP_TZ"];
