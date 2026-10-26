//! The literal date formats TRY_TO_DATE converts.

use crate::temporal::models::DateFormat;

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
        pattern: "^([0-9]{2})/([0-9]{2})/([0-9]{4})$",
        groups: (3, 2, 1),
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
