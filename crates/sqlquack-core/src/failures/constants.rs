//! Which Snowflake errors and DuckDB messages mean which kind of failure.

use crate::failures::models::{ErrorKind, FailureMode, SnowflakeCode};

/// The prefix building blocks put before a Snowflake error number in the errors they raise.
pub const ERROR_TAG_PREFIX: &str = "sqlquack:";
/// What separates a raised error's tag from the ID of the expression raising it.
pub const RAISE_ID_MARKER: char = '#';
/// The ID a raise carries until the expression it guards is converted and identifies it.
pub const UNIDENTIFIED_RAISE: &str = "?";
/// How many hexadecimal digits a raise ID has.
pub const RAISE_ID_DIGITS: usize = 12;
/// What ends a raised error's tag, before its message.
pub const TAG_END: &str = ": ";

/// Numeric value out of representable range.
pub const NUMBER_OUT_OF_RANGE: SnowflakeCode = SnowflakeCode {
    errno: 100_046,
    sqlstate: "22003",
};
/// Numeric value out of range for the target type.
pub const NUMBER_TEXT_OUT_OF_RANGE: SnowflakeCode = SnowflakeCode {
    errno: 100_039,
    sqlstate: "22003",
};
/// Numeric value is not recognized.
pub const NUMBER_NOT_RECOGNIZED: SnowflakeCode = SnowflakeCode {
    errno: 100_038,
    sqlstate: "22018",
};
/// Boolean value is not recognized.
pub const BOOLEAN_NOT_RECOGNIZED: SnowflakeCode = SnowflakeCode {
    errno: 100_037,
    sqlstate: "22018",
};
/// Date is not recognized.
pub const DATE_NOT_RECOGNIZED: SnowflakeCode = SnowflakeCode {
    errno: 100_040,
    sqlstate: "22007",
};
/// Can't parse the text as a date with the format.
pub const DATE_FORMAT_MISMATCH: SnowflakeCode = SnowflakeCode {
    errno: 100_097,
    sqlstate: "22007",
};
/// Timestamp is not recognized.
pub const TIMESTAMP_NOT_RECOGNIZED: SnowflakeCode = SnowflakeCode {
    errno: 100_035,
    sqlstate: "22007",
};
/// Can't parse the text as a timestamp with the format.
pub const TIMESTAMP_FORMAT_MISMATCH: SnowflakeCode = SnowflakeCode {
    errno: 100_096,
    sqlstate: "22007",
};
/// Time is not recognized.
pub const TIME_NOT_RECOGNIZED: SnowflakeCode = SnowflakeCode {
    errno: 100_108,
    sqlstate: "22007",
};
/// Division by zero.
pub const DIVISION_BY_ZERO: SnowflakeCode = SnowflakeCode {
    errno: 100_051,
    sqlstate: "22012",
};
/// Error parsing JSON.
pub const JSON_NOT_PARSED: SnowflakeCode = SnowflakeCode {
    errno: 100_069,
    sqlstate: "22P02",
};
/// Failed to cast a variant value.
pub const VARIANT_NOT_CAST: SnowflakeCode = SnowflakeCode {
    errno: 100_071,
    sqlstate: "22000",
};
/// String is too long and would be truncated.
pub const STRING_TOO_LONG: SnowflakeCode = SnowflakeCode {
    errno: 100_078,
    sqlstate: "22000",
};
/// Bytes longer than a BINARY(n) target, which Snowflake does not truncate.
pub const BINARY_TOO_LONG: SnowflakeCode = SnowflakeCode {
    errno: 100_145,
    sqlstate: "22000",
};

/// A function's array argument is not an array.
pub const NOT_AN_ARRAY_ARGUMENT: SnowflakeCode = SnowflakeCode {
    errno: 100_098,
    sqlstate: "22000",
};

/// Text that does not match a TIME format.
pub const TIME_FORMAT_MISMATCH: SnowflakeCode = SnowflakeCode {
    errno: 100_136,
    sqlstate: "22007",
};

/// A time zone name no zone has.
pub const UNKNOWN_TIME_ZONE: SnowflakeCode = SnowflakeCode {
    errno: 100_094,
    sqlstate: "22000",
};

/// A code point no character has, given to CHR.
pub const INVALID_CHARACTER_CODE: SnowflakeCode = SnowflakeCode {
    errno: 100_062,
    sqlstate: "22021",
};

/// A regular expression Snowflake cannot read, such as a repetition operator with no argument.
pub const INVALID_REGULAR_EXPRESSION: SnowflakeCode = SnowflakeCode {
    errno: 100_048,
    sqlstate: "2201B",
};

/// The object already holds the key.
pub const OBJECT_KEY_EXISTS: SnowflakeCode = SnowflakeCode {
    errno: 100_103,
    sqlstate: "22000",
};

/// Bytes that are not UTF-8, read as UTF-8 text.
pub const INVALID_UTF8: SnowflakeCode = SnowflakeCode {
    errno: 100_146,
    sqlstate: "22000",
};

/// A floating point operation outside its domain, such as the logarithm of zero.
pub const INVALID_FLOAT_OPERATION: SnowflakeCode = SnowflakeCode {
    errno: 100_043,
    sqlstate: "2201E",
};

/// ARRAY_FLATTEN of an array holding an element other than an array.
pub const FLATTEN_ELEMENT_NOT_ARRAY: SnowflakeCode = SnowflakeCode {
    errno: 100_107,
    sqlstate: "22000",
};

/// ARRAY_GENERATE_RANGE with a step of zero.
pub const ZERO_RANGE_STEP: SnowflakeCode = SnowflakeCode {
    errno: 100_050,
    sqlstate: "22023",
};

/// HEX_DECODE_STRING of text that is not hexadecimal digits of UTF-8 bytes.
pub const HEX_DECODE_FAILED: SnowflakeCode = SnowflakeCode {
    errno: 100_115,
    sqlstate: "22000",
};

/// The square root of a negative number.
pub const SQUARE_ROOT_OF_NEGATIVE: SnowflakeCode = SnowflakeCode {
    errno: 100_044,
    sqlstate: "22P01",
};

/// Every Snowflake error the recordings hold, and its kind.
pub const SNOWFLAKE_ERRORS: &[(SnowflakeCode, ErrorKind)] = &[
    (NUMBER_OUT_OF_RANGE, ErrorKind::NumericOverflow),
    (NUMBER_TEXT_OUT_OF_RANGE, ErrorKind::NumericOverflow),
    (NUMBER_NOT_RECOGNIZED, ErrorKind::NumberText),
    (BOOLEAN_NOT_RECOGNIZED, ErrorKind::BooleanText),
    (DATE_NOT_RECOGNIZED, ErrorKind::DateText),
    (DATE_FORMAT_MISMATCH, ErrorKind::DateText),
    (TIMESTAMP_NOT_RECOGNIZED, ErrorKind::TimestampText),
    (TIMESTAMP_FORMAT_MISMATCH, ErrorKind::TimestampText),
    (TIME_NOT_RECOGNIZED, ErrorKind::TimeText),
    (TIME_FORMAT_MISMATCH, ErrorKind::TimeText),
    (DIVISION_BY_ZERO, ErrorKind::DivisionByZero),
    (JSON_NOT_PARSED, ErrorKind::JsonParse),
    (VARIANT_NOT_CAST, ErrorKind::VariantCast),
    (STRING_TOO_LONG, ErrorKind::StringTooLong),
    (BINARY_TOO_LONG, ErrorKind::StringTooLong),
    (NOT_AN_ARRAY_ARGUMENT, ErrorKind::InvalidArgument),
    (OBJECT_KEY_EXISTS, ErrorKind::InvalidArgument),
    (INVALID_CHARACTER_CODE, ErrorKind::InvalidArgument),
    (INVALID_REGULAR_EXPRESSION, ErrorKind::InvalidArgument),
    (INVALID_UTF8, ErrorKind::InvalidArgument),
    (INVALID_FLOAT_OPERATION, ErrorKind::InvalidArgument),
    (FLATTEN_ELEMENT_NOT_ARRAY, ErrorKind::InvalidArgument),
    (ZERO_RANGE_STEP, ErrorKind::InvalidArgument),
    (HEX_DECODE_FAILED, ErrorKind::InvalidArgument),
    (SQUARE_ROOT_OF_NEGATIVE, ErrorKind::InvalidArgument),
    (UNKNOWN_TIME_ZONE, ErrorKind::InvalidArgument),
    (
        SnowflakeCode {
            errno: 1007,
            sqlstate: "22023",
        },
        ErrorKind::Compilation,
    ),
    (
        SnowflakeCode {
            errno: 1003,
            sqlstate: "42000",
        },
        ErrorKind::Compilation,
    ),
    (
        SnowflakeCode {
            errno: 2031,
            sqlstate: "42601",
        },
        ErrorKind::Compilation,
    ),
];

/// Fragments of DuckDB's own messages, in the order they are tried, and their kinds.
pub const DUCKDB_MESSAGE_KINDS: &[(&str, ErrorKind)] = &[
    ("date field value out of range", ErrorKind::DateText),
    ("Overflow", ErrorKind::NumericOverflow),
    ("out of range", ErrorKind::NumericOverflow),
    ("Out of Range Error", ErrorKind::NumericOverflow),
    ("to BOOL", ErrorKind::BooleanText),
    ("to DATE", ErrorKind::DateText),
    ("to TIMESTAMP", ErrorKind::TimestampText),
    ("to TIME", ErrorKind::TimeText),
    ("Failed to cast decimal value", ErrorKind::NumericOverflow),
    ("Could not cast value", ErrorKind::NumericOverflow),
    ("Could not convert string", ErrorKind::NumberText),
    ("Malformed JSON", ErrorKind::JsonParse),
    ("Division by zero", ErrorKind::DivisionByZero),
    ("Unknown TimeZone", ErrorKind::InvalidArgument),
    ("Binder Error", ErrorKind::Compilation),
    ("Parser Error", ErrorKind::Compilation),
];

/// How dividing by zero fails.
pub const DIVISION_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::DivisionByZero,
    snowflake: &[DIVISION_BY_ZERO],
}];

/// How reading text as a TIME fails, without and with a format.
pub const TIME_TEXT_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::TimeText,
    snowflake: &[TIME_NOT_RECOGNIZED, TIME_FORMAT_MISMATCH],
}];

/// How LN fails for zero or a negative number, and SQRT for a negative one.
pub const LOGARITHM_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[INVALID_FLOAT_OPERATION, SQUARE_ROOT_OF_NEGATIVE],
}];

/// How ARRAY_FLATTEN fails for an element other than an array.
pub const FLATTEN_FAILURES: &[FailureMode] = &[
    FailureMode {
        kind: ErrorKind::InvalidArgument,
        snowflake: &[FLATTEN_ELEMENT_NOT_ARRAY],
    },
    FailureMode {
        kind: ErrorKind::InvalidArgument,
        snowflake: &[NOT_AN_ARRAY_ARGUMENT],
    },
];

/// How ARRAY_GENERATE_RANGE fails for a step of zero.
pub const RANGE_STEP_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[ZERO_RANGE_STEP],
}];

/// How HEX_DECODE_STRING fails for text that is not hexadecimal UTF-8.
pub const HEX_DECODE_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[HEX_DECODE_FAILED],
}];

/// How CHR fails for a code point no character has.
pub const CHARACTER_CODE_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[INVALID_CHARACTER_CODE],
}];

/// How reading bytes as UTF-8 text fails for bytes that are not UTF-8.
pub const INVALID_UTF8_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[INVALID_UTF8],
}];

/// How an array function fails for an argument other than an array.
pub const NOT_AN_ARRAY_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[NOT_AN_ARRAY_ARGUMENT],
}];

/// How OBJECT_AGG fails for a repeated key.
pub const KEY_EXISTS_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[OBJECT_KEY_EXISTS],
}];

/// How a time zone conversion fails for a name no zone has.
pub const ZONE_FAILURES: &[FailureMode] = &[FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[UNKNOWN_TIME_ZONE],
}];
