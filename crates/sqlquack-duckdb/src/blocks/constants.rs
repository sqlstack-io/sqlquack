//! Building block identifiers and the data they use.

/// DuckDB type names: `duckdb.type`.
pub const TYPE_NAME: &str = "duckdb.type";
/// Explicit casts: `duckdb.cast`.
pub const CAST: &str = "duckdb.cast";
/// Truncating timestamp precision changes: `duckdb.truncate_timestamp`.
pub const TRUNCATE_TIMESTAMP: &str = "duckdb.truncate_timestamp";
/// Run-time checks that a nanosecond timestamp has no sub-microsecond part: `duckdb.submicro_check`.
pub const SUBMICRO_CHECK: &str = "duckdb.submicro_check";
/// The error a zoned timestamp with a sub-microsecond part raises.
pub const SUBMICRO_MESSAGE: &str =
    "timestamp_tz_submicro: a zoned timestamp keeps nanoseconds DuckDB cannot hold";
/// Digits of a nanosecond timestamp's fraction.
pub const NANOSECOND_DIGITS: u8 = 9;
/// Date and timestamp parts: `duckdb.date_part`.
pub const DATE_PART: &str = "duckdb.date_part";
/// Date and timestamp truncation to a unit: `duckdb.date_trunc`.
pub const DATE_TRUNC: &str = "duckdb.date_trunc";
/// Adding calendar or clock units: `duckdb.date_add`.
pub const DATE_ADD: &str = "duckdb.date_add";
/// A DATEADD amount checked against the magnitude Snowflake rejects: `duckdb.date_add_amount`.
pub const DATE_ADD_AMOUNT: &str = "duckdb.date_add_amount";
/// The largest DATEADD amount magnitude recorded short of Snowflake's error 100046: 2^53.
pub const DATE_ADD_AMOUNT_LIMIT: &str = "9007199254740992";
/// Calendar months a DATEADD may move a value, well inside DuckDB's interval and date ranges.
pub const DATE_ADD_MONTH_LIMIT: i64 = 2_400_000;
/// Days a DATEADD may move a value, well inside DuckDB's interval and date ranges.
pub const DATE_ADD_DAY_LIMIT: i64 = 73_000_000;
/// DuckDB's microsecond timestamp range, in microseconds either side of the epoch.
pub const MICROSECOND_TIMESTAMP_LIMIT: i64 = 9_223_372_036_854_775_806;
/// The microseconds a nanosecond timestamp can hold either side of the epoch.
pub const NANOSECOND_TIMESTAMP_MICROS: i64 = 9_223_372_036_854_774;
/// Counting unit boundaries between two values: `duckdb.date_diff`.
pub const DATE_DIFF: &str = "duckdb.date_diff";
/// Naive UTC wall times as wall times in another zone: `duckdb.utc_to_zone`.
pub const UTC_TO_ZONE: &str = "duckdb.utc_to_zone";
/// Naive wall times in one zone as wall times in another, repeated times read early: `duckdb.zone_to_zone`.
pub const ZONE_TO_ZONE: &str = "duckdb.zone_to_zone";
/// Zoned instants as naive UTC wall times: `duckdb.utc_wall_time`.
pub const UTC_WALL_TIME: &str = "duckdb.utc_wall_time";
/// Run-time checks that a time zone name exists: `duckdb.zone_checked`.
pub const ZONE_CHECKED: &str = "duckdb.zone_checked";
/// Text parsed as a timestamp or date in the recorded spellings: `duckdb.text_to_timestamp`.
pub const TEXT_TO_TIMESTAMP: &str = "duckdb.text_to_timestamp";
/// Text in one literal date format as a date: `duckdb.formatted_text_to_date`.
pub const FORMATTED_TEXT_TO_DATE: &str = "duckdb.formatted_text_to_date";
/// Text in a literal timestamp format: `duckdb.formatted_text_to_timestamp`.
pub const FORMATTED_TEXT_TO_TIMESTAMP: &str = "duckdb.formatted_text_to_timestamp";
/// The message of the error TO_DATE raises for an impossible date in its format.
pub const UNPARSED_DATE_MESSAGE: &str = "Can't parse the text as date with the format";
/// The first year two-digit years are read in, Snowflake's default TWO_DIGIT_CENTURY_START.
pub const TWO_DIGIT_CENTURY_START: u16 = 1970;
/// Epoch counts as naive timestamps: `duckdb.epoch_to_timestamp`.
pub const EPOCH_TO_TIMESTAMP: &str = "duckdb.epoch_to_timestamp";
/// A date with one- or two-digit fields and an optional unpadded clock time.
pub const UNPADDED_TIMESTAMP_PATTERN: &str =
    "[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}(?: [0-9]{1,2}:[0-9]{1,2}:[0-9]{1,2})?";
/// A month-first date and a clock time.
pub const MONTH_FIRST_TIMESTAMP_PATTERN: &str =
    "[0-9]{2}/[0-9]{2}/[0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2}";
/// An RFC 2822 date-time at offset +0000.
pub const RFC_UTC_TIMESTAMP_PATTERN: &str = "(Mon|Tue|Wed|Thu|Fri|Sat|Sun), [0-9]{2} (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} [+]0000";
/// English month abbreviations, upper case, January first.
pub const MONTH_ABBREVIATIONS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];
/// Seconds in a day, for epoch integers read as dates.
pub const SECONDS_PER_DAY: i64 = 86_400;
/// A month-first date, as Snowflake's AUTO input format reads it.
pub const MONTH_FIRST_DATE_PATTERN: &str = "[0-9]{2}/[0-9]{2}/[0-9]{4}";
/// A day, an English month abbreviation in title or upper case and a 2- or 4-digit year, captured.
pub const DAY_MONTH_NAME_DATE_PATTERN: &str = "([0-9]{1,2})-(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|JAN|FEB|MAR|APR|MAY|JUN|JUL|AUG|SEP|OCT|NOV|DEC)-([0-9]{4}|[0-9]{2})";
/// Spellings TRY_ conversions to DATE were recorded returning NULL for.
pub const NULL_DATE_TEXT_PATTERN: &str = "[0-9]{4}/[0-9]{2}/[0-9]{2}|(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{1,2}, [0-9]{4}";
/// The text closing every raise; SQL holding it holds a raise (a false match is harmless).
pub const RAISE_MARKER: &str = "' AS INTEGER) AS \"NULL\")";
/// The known divergence for timestamps DuckDB cannot hold in nanoseconds.
pub const TIMESTAMP_RANGE: &str = "timestamp_range";
/// The error a TRY_ conversion to DATE raises for a spelling no recording shows.
pub const DATE_TEXT_SPELLING: &str = "date_text_spelling";
/// ISO date-time text with an optional `T`, fraction and Z, ±HH:MM or ` ±HHMM` zone, captured.
pub const ISO_TIMESTAMP_PATTERN: &str = "([0-9]{4}-[0-9]{2}-[0-9]{2})(?:[ T](([01][0-9]|2[0-3]):[0-5][0-9])(?:(:[0-5][0-9])(?:[.]([0-9]+))?)?)?(Z| ?[+-][0-9]{2}:[0-9]{2}| [+-][0-9]{4})?";
/// Text without an ASCII digit, which TRY_ date and timestamp parsing were recorded reading as NULL.
pub const DIGITLESS_TEXT_PATTERN: &str = "[^0-9]*";
/// Spellings TRY_ conversions to timestamps were recorded returning NULL for.
pub const NULL_TIMESTAMP_TEXT_PATTERN: &str = "[0-9]{4}-[0-9]{2}-[0-9]{2}(?:T[0-9]{2}:[0-9]{2}(?::[0-9]{2}(?:[.][0-9]+)?)?[+-][0-9]{4}|t[0-9]{2}:[0-9]{2}:[0-9]{2}(?:[.][0-9]+)?z?|T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:[.][0-9]+)?z)|[0-9]{5}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2}";
/// The zone group of ISO_TIMESTAMP_PATTERN: sign, hours and minutes of an offset.
pub const ZONE_OFFSET_PATTERN: &str = "([+-])([0-9]{2}):?([0-9]{2})";
/// The largest offset hour Snowflake was recorded reading, and the largest minute.
pub const ZONE_OFFSET_LIMITS: (u8, u8) = (25, 59);
/// Text to a zoned timestamp's instant as UTC wall time: `duckdb.text_to_zoned`.
pub const TEXT_TO_ZONED: &str = "duckdb.text_to_zoned";
/// Non-negative integer text, read as an epoch count scaled by its magnitude.
pub const EPOCH_TEXT_PATTERN: &str = "[0-9]{1,18}";
/// Epoch magnitudes below which integer text counts seconds, milliseconds and microseconds.
pub const EPOCH_SCALE_LIMITS: [i64; 3] =
    [31_536_000_000, 31_536_000_000_000, 31_536_000_000_000_000];
/// Quoted identifiers: `duckdb.quoted_identifier`.
pub const QUOTED_IDENTIFIER: &str = "duckdb.quoted_identifier";
/// String literals: `duckdb.string_literal`.
pub const STRING_LITERAL: &str = "duckdb.string_literal";
/// Exact `+ - *` at 38 digits, then the Snowflake result type: `duckdb.decimal_arithmetic`.
pub const DECIMAL_ARITHMETIC: &str = "duckdb.decimal_arithmetic";
/// Double arithmetic that raises on division by zero: `duckdb.double_arithmetic`.
pub const DOUBLE_ARITHMETIC: &str = "duckdb.double_arithmetic";
/// Exact decimal division rounding half away from zero: `duckdb.exact_decimal_divide`.
pub const EXACT_DECIMAL_DIVIDE: &str = "duckdb.exact_decimal_divide";
/// Raising an error for text longer than a VARCHAR(n) target: `duckdb.varchar_length_check`.
pub const VARCHAR_LENGTH_CHECK: &str = "duckdb.varchar_length_check";
/// NULL for text longer than a TRY_CAST's VARCHAR(n) target: `duckdb.varchar_length_try`.
pub const VARCHAR_LENGTH_TRY: &str = "duckdb.varchar_length_try";
/// Full Unicode case mapping, expanding characters such as ß: `duckdb.full_case_mapping`.
pub const FULL_CASE_MAPPING: &str = "duckdb.full_case_mapping";
/// Concatenation that is NULL when any operand is: `duckdb.null_propagating_concat`.
pub const NULL_PROPAGATING_CONCAT: &str = "duckdb.null_propagating_concat";
/// Sort keys with Snowflake's NULL placement spelled out: `duckdb.explicit_null_order`.
pub const EXPLICIT_NULL_ORDER: &str = "duckdb.explicit_null_order";
/// Window frames spelled out: `duckdb.window_frame`.
pub const WINDOW_FRAME: &str = "duckdb.window_frame";
/// Text to NUMBER with Snowflake's accepted spellings: `duckdb.text_to_number`.
pub const TEXT_TO_NUMBER: &str = "duckdb.text_to_number";
/// Text to FLOAT with Snowflake's accepted spellings: `duckdb.text_to_float`.
pub const TEXT_TO_FLOAT: &str = "duckdb.text_to_float";
/// Text to BOOLEAN with Snowflake's accepted spellings: `duckdb.text_to_boolean`.
pub const TEXT_TO_BOOLEAN: &str = "duckdb.text_to_boolean";
/// BOOLEAN to and from NUMBER: `duckdb.boolean_number`.
pub const BOOLEAN_NUMBER: &str = "duckdb.boolean_number";

/// Fractional-second digits a DuckDB TIMESTAMP or TIME keeps.
pub const DUCKDB_FRACTIONAL_PRECISION: u8 = 6;
/// Seconds from the epoch to 0001-01-01 00:00:00, the first instant Snowflake writes.
pub const EPOCH_SECONDS_MIN: i128 = -62_135_596_800;
/// Seconds from the epoch to 9999-12-31 23:59:59, the last whole second Snowflake writes.
pub const EPOCH_SECONDS_MAX: i128 = 253_402_300_799;
/// Decimal text Snowflake reads as a number: sign, digits with an optional point, exponent.
pub const DECIMAL_TEXT_PATTERN: &str = "[+-]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eE][+-]?[0-9]+)?";
/// Decimal text with an exponent marker but no exponent digits, which Snowflake reads as exponent 0.
pub const BARE_EXPONENT_PATTERN: &str = "[+-]?([0-9]+[.]?[0-9]*|[.][0-9]+)[eE]";
/// An exponent without a mantissa, which Snowflake reads as zero.
pub const MANTISSA_FREE_PATTERN: &str = "[eE][+-]?[0-9]+";
/// The longest argument a block repeats inline; a longer one is bound once, so nesting stays linear.
pub const INLINE_ARGUMENT_LIMIT: usize = 256;
/// Whitespace conversions ignore around a value (space, tab, line feed; not no-break space).
pub const CONVERSION_WHITESPACE: &str = "(' ' || chr(9) || chr(10))";
/// Words Snowflake reads as a non-finite FLOAT, lower-cased, with their DuckDB spelling.
pub const NON_FINITE_FLOAT_WORDS: &[(&str, &str)] = &[
    ("nan", "nan"),
    ("-nan", "nan"),
    ("inf", "inf"),
    ("+inf", "inf"),
    ("infinity", "inf"),
    ("-inf", "-inf"),
    ("-infinity", "-inf"),
];
/// Text Snowflake reads as TRUE, lower-cased after trimming whitespace.
pub const TRUE_WORDS: &[&str] = &["true", "t", "yes", "y", "on", "1"];
/// Text Snowflake reads as FALSE, lower-cased after trimming whitespace.
pub const FALSE_WORDS: &[&str] = &["false", "f", "no", "n", "off", "0"];
/// DuckDB's largest DECIMAL precision.
pub const MAX_DECIMAL_PRECISION: u8 = 38;
/// Characters whose full uppercase mapping has several characters.
pub const UPPERCASE_EXPANSIONS: &[(&str, &str)] = &[
    ("\u{df}", "SS"),
    ("\u{149}", "\u{2bc}N"),
    ("\u{1f0}", "J\u{30c}"),
    ("\u{390}", "\u{399}\u{308}\u{301}"),
    ("\u{3b0}", "\u{3a5}\u{308}\u{301}"),
    ("\u{587}", "\u{535}\u{552}"),
    ("\u{1e96}", "H\u{331}"),
    ("\u{1e97}", "T\u{308}"),
    ("\u{1e98}", "W\u{30a}"),
    ("\u{1e99}", "Y\u{30a}"),
    ("\u{1e9a}", "A\u{2be}"),
    ("\u{1f50}", "\u{3a5}\u{313}"),
    ("\u{1f52}", "\u{3a5}\u{313}\u{300}"),
    ("\u{1f54}", "\u{3a5}\u{313}\u{301}"),
    ("\u{1f56}", "\u{3a5}\u{313}\u{342}"),
    ("\u{1f80}", "\u{1f08}\u{399}"),
    ("\u{1f81}", "\u{1f09}\u{399}"),
    ("\u{1f82}", "\u{1f0a}\u{399}"),
    ("\u{1f83}", "\u{1f0b}\u{399}"),
    ("\u{1f84}", "\u{1f0c}\u{399}"),
    ("\u{1f85}", "\u{1f0d}\u{399}"),
    ("\u{1f86}", "\u{1f0e}\u{399}"),
    ("\u{1f87}", "\u{1f0f}\u{399}"),
    ("\u{1f88}", "\u{1f08}\u{399}"),
    ("\u{1f89}", "\u{1f09}\u{399}"),
    ("\u{1f8a}", "\u{1f0a}\u{399}"),
    ("\u{1f8b}", "\u{1f0b}\u{399}"),
    ("\u{1f8c}", "\u{1f0c}\u{399}"),
    ("\u{1f8d}", "\u{1f0d}\u{399}"),
    ("\u{1f8e}", "\u{1f0e}\u{399}"),
    ("\u{1f8f}", "\u{1f0f}\u{399}"),
    ("\u{1f90}", "\u{1f28}\u{399}"),
    ("\u{1f91}", "\u{1f29}\u{399}"),
    ("\u{1f92}", "\u{1f2a}\u{399}"),
    ("\u{1f93}", "\u{1f2b}\u{399}"),
    ("\u{1f94}", "\u{1f2c}\u{399}"),
    ("\u{1f95}", "\u{1f2d}\u{399}"),
    ("\u{1f96}", "\u{1f2e}\u{399}"),
    ("\u{1f97}", "\u{1f2f}\u{399}"),
    ("\u{1f98}", "\u{1f28}\u{399}"),
    ("\u{1f99}", "\u{1f29}\u{399}"),
    ("\u{1f9a}", "\u{1f2a}\u{399}"),
    ("\u{1f9b}", "\u{1f2b}\u{399}"),
    ("\u{1f9c}", "\u{1f2c}\u{399}"),
    ("\u{1f9d}", "\u{1f2d}\u{399}"),
    ("\u{1f9e}", "\u{1f2e}\u{399}"),
    ("\u{1f9f}", "\u{1f2f}\u{399}"),
    ("\u{1fa0}", "\u{1f68}\u{399}"),
    ("\u{1fa1}", "\u{1f69}\u{399}"),
    ("\u{1fa2}", "\u{1f6a}\u{399}"),
    ("\u{1fa3}", "\u{1f6b}\u{399}"),
    ("\u{1fa4}", "\u{1f6c}\u{399}"),
    ("\u{1fa5}", "\u{1f6d}\u{399}"),
    ("\u{1fa6}", "\u{1f6e}\u{399}"),
    ("\u{1fa7}", "\u{1f6f}\u{399}"),
    ("\u{1fa8}", "\u{1f68}\u{399}"),
    ("\u{1fa9}", "\u{1f69}\u{399}"),
    ("\u{1faa}", "\u{1f6a}\u{399}"),
    ("\u{1fab}", "\u{1f6b}\u{399}"),
    ("\u{1fac}", "\u{1f6c}\u{399}"),
    ("\u{1fad}", "\u{1f6d}\u{399}"),
    ("\u{1fae}", "\u{1f6e}\u{399}"),
    ("\u{1faf}", "\u{1f6f}\u{399}"),
    ("\u{1fb2}", "\u{1fba}\u{399}"),
    ("\u{1fb3}", "\u{391}\u{399}"),
    ("\u{1fb4}", "\u{386}\u{399}"),
    ("\u{1fb6}", "\u{391}\u{342}"),
    ("\u{1fb7}", "\u{391}\u{342}\u{399}"),
    ("\u{1fbc}", "\u{391}\u{399}"),
    ("\u{1fc2}", "\u{1fca}\u{399}"),
    ("\u{1fc3}", "\u{397}\u{399}"),
    ("\u{1fc4}", "\u{389}\u{399}"),
    ("\u{1fc6}", "\u{397}\u{342}"),
    ("\u{1fc7}", "\u{397}\u{342}\u{399}"),
    ("\u{1fcc}", "\u{397}\u{399}"),
    ("\u{1fd2}", "\u{399}\u{308}\u{300}"),
    ("\u{1fd3}", "\u{399}\u{308}\u{301}"),
    ("\u{1fd6}", "\u{399}\u{342}"),
    ("\u{1fd7}", "\u{399}\u{308}\u{342}"),
    ("\u{1fe2}", "\u{3a5}\u{308}\u{300}"),
    ("\u{1fe3}", "\u{3a5}\u{308}\u{301}"),
    ("\u{1fe4}", "\u{3a1}\u{313}"),
    ("\u{1fe6}", "\u{3a5}\u{342}"),
    ("\u{1fe7}", "\u{3a5}\u{308}\u{342}"),
    ("\u{1ff2}", "\u{1ffa}\u{399}"),
    ("\u{1ff3}", "\u{3a9}\u{399}"),
    ("\u{1ff4}", "\u{38f}\u{399}"),
    ("\u{1ff6}", "\u{3a9}\u{342}"),
    ("\u{1ff7}", "\u{3a9}\u{342}\u{399}"),
    ("\u{1ffc}", "\u{3a9}\u{399}"),
    ("\u{fb00}", "FF"),
    ("\u{fb01}", "FI"),
    ("\u{fb02}", "FL"),
    ("\u{fb03}", "FFI"),
    ("\u{fb04}", "FFL"),
    ("\u{fb05}", "ST"),
    ("\u{fb06}", "ST"),
    ("\u{fb13}", "\u{544}\u{546}"),
    ("\u{fb14}", "\u{544}\u{535}"),
    ("\u{fb15}", "\u{544}\u{53b}"),
    ("\u{fb16}", "\u{54e}\u{546}"),
    ("\u{fb17}", "\u{544}\u{53d}"),
];
/// Characters whose full lowercase mapping has several characters.
pub const LOWERCASE_EXPANSIONS: &[(&str, &str)] = &[("\u{130}", "i\u{307}")];
/// The message of the error a failed VARIANT cast raises.
pub const VARIANT_CAST_MESSAGE: &str = "Failed to cast variant value";
/// The message of the error numeric text beyond its type's range raises.
pub const NUMBER_RANGE_MESSAGE: &str = "Numeric value out of range";
