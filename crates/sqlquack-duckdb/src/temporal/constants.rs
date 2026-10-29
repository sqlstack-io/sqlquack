//! The literal date formats TRY_TO_DATE converts and the timestamp format elements read.

use crate::temporal::models::{DateFormat, TimestampField};

/// Each supported format, matched case-sensitively as written.
pub const DATE_FORMATS: &[DateFormat] = &[
    DateFormat {
        snowflake: "YYYYMMDD",
        pattern: "^([0-9]{4})([0-9]{2})([0-9]{2})$",
        groups: (1, 2, 3),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "YYYY-MM-DD",
        pattern: "^([0-9]{4})-([0-9]{2})-([0-9]{2})$",
        groups: (1, 2, 3),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "DD.MM.YYYY",
        pattern: r"^([0-9]{2})\.([0-9]{2})\.([0-9]{4})$",
        groups: (3, 2, 1),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "DD.MM.YY",
        pattern: r"^([0-9]{2})\.([0-9]{2})\.([0-9]{2})$",
        groups: (3, 2, 1),
        two_digit_year: true,
    },
    DateFormat {
        snowflake: "YY-MM-DD",
        pattern: "^([0-9]{2})-([0-9]{2})-([0-9]{2})$",
        groups: (1, 2, 3),
        two_digit_year: true,
    },
    DateFormat {
        snowflake: "DD-MM-YY",
        pattern: "^([0-9]{2})-([0-9]{2})-([0-9]{2})$",
        groups: (3, 2, 1),
        two_digit_year: true,
    },
    DateFormat {
        snowflake: "DD/MM/YYYY",
        pattern: "^([0-9]{1,2})/([0-9]{1,2})/([0-9]{4})$",
        groups: (3, 2, 1),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "DD-MM-YYYY",
        pattern: "^ *([0-9]{1,2})-([0-9]{1,2})-([0-9]{4})$",
        groups: (3, 2, 1),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "DD. MM. YYYY",
        pattern: r"^([0-9]{1,2})\. ?([0-9]{1,2})\. ?([0-9]{4})$",
        groups: (3, 2, 1),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "YYYY.MM.DD",
        pattern: r"^([0-9]{4})\.([0-9]{1,2})\.([0-9]{1,2})$",
        groups: (1, 2, 3),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "MM/DD/YYYY",
        pattern: "^([0-9]{2})/([0-9]{2})/([0-9]{4})$",
        groups: (3, 1, 2),
        two_digit_year: false,
    },
    DateFormat {
        snowflake: "YYYY/MM/DD",
        pattern: "^([0-9]{4})/([0-9]{2})/([0-9]{2})$",
        groups: (1, 2, 3),
        two_digit_year: false,
    },
];

/// Nanoseconds in a minute, for zone offsets.
pub const NANOS_PER_MINUTE: i64 = 60_000_000_000;

/// Letters a timestamp format may hold as literal text: none begins a Snowflake format element.
pub const LITERAL_FORMAT_LETTERS: &str = "BCEGIJKLNOQRVWXZbcegijklnoqrvwxz";

/// Punctuation a timestamp format may hold as literal text.
pub const LITERAL_FORMAT_PUNCTUATION: &str = "-:./,";

/// Format elements in matching order, each with its field and the text pattern it reads.
pub const TIMESTAMP_FORMAT_ELEMENTS: &[(&str, TimestampField, &str)] = &[
    ("YYYY", TimestampField::Year, "([0-9]{4})"),
    ("HH24", TimestampField::Hour24, "([0-9]{1,2})"),
    ("HH12", TimestampField::Hour12, "([0-9]{1,2})"),
    ("MM", TimestampField::Month, "([0-9]{1,2})"),
    ("DD", TimestampField::Day, "([0-9]{1,2})"),
    ("MI", TimestampField::Minute, "([0-9]{1,2})"),
    ("SS", TimestampField::Second, "([0-9]{1,2})"),
    ("AM", TimestampField::Meridiem, "([AaPp][Mm])"),
    ("PM", TimestampField::Meridiem, "([AaPp][Mm])"),
];

/// A DATE from year, month and day numbers, normalising overflow: `duckdb.date_from_parts`.
pub const DATE_FROM_PARTS: &str = "duckdb.date_from_parts";
/// A nanosecond timestamp from a date and a time of day: `duckdb.timestamp_from_parts`.
pub const TIMESTAMP_FROM_PARTS: &str = "duckdb.timestamp_from_parts";
/// The dates whose every instant DuckDB's nanosecond timestamps hold.
pub const NANOSECOND_DATE_RANGE: (&str, &str) = ("1677-09-22", "2262-04-10");
