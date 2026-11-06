//! Format elements: their strftime spellings, the month names, fractions and separators.

/// Format fields: the Snowflake token, its strftime spelling, and whether it is a time field.
pub(crate) const TEMPORAL_TEXT_FIELDS: &[(&str, &str, bool)] = &[
    ("YYYY", "%Y", false),
    ("MMMM", "%B", false),
    ("MON", "%b", false),
    ("MM", "%m", false),
    ("DD", "%d", false),
    ("HH24", "%H", true),
    ("MI", "%M", true),
    ("SS", "%S", true),
    ("FF9", "%n", true),
    ("FF6", "%f", true),
    ("FF3", "%g", true),
];
/// Fields writing an English month name, recorded only as written in upper case.
pub(crate) const MONTH_NAME_FIELDS: &[&str] = &["MMMM", "MON"];
/// The fields recorded directly after each month name: none after MON, whose MONMM writes the full name.
pub(crate) const MONTH_NAME_FOLLOWERS: &[(&str, &[&str])] = &[("MON", &[]), ("MMMM", &["DD"])];
/// The format element writing as many fraction digits as the timestamp's precision.
pub(crate) const BARE_FRACTION: &str = "FF";
/// What FF writes for each precision recorded: none for 0, then milli-, micro- and nanoseconds.
pub(crate) const BARE_FRACTION_PATTERNS: &[(u8, &str)] =
    &[(0, ""), (3, "%g"), (6, "%f"), (9, "%n")];
/// Snowflake's TIMESTAMP_NTZ output format, YYYY-MM-DD HH24:MI:SS.FF3, as a strftime pattern.
pub(crate) const NTZ_OUTPUT_PATTERN: &str = "%Y-%m-%d %H:%M:%S.%g";
/// Characters a format may hold between fields, written as they are.
pub(crate) const TEMPORAL_TEXT_SEPARATORS: &str = "-_/:. ";
/// The pieces of a TIME format recorded through TO_VARCHAR: clock fields and separators.
pub(crate) const CLOCK_FORMAT_FIELDS: &[&str] = &[
    "HH24", "MI", "SS", "FF1", "FF2", "FF3", "FF4", "FF5", "FF6", ":", ".",
];
