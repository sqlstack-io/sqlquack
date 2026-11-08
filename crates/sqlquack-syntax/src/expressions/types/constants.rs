//! Type names with fixed meanings in Snowflake SQL.

/// Type names Snowflake refuses before a string, each with the conversion function to suggest.
pub const SNOWFLAKE_UNTYPED_LITERAL_TYPES: &[(&str, &str)] = &[
    ("TIMESTAMP_NTZ", "TO_TIMESTAMP_NTZ"),
    ("TIMESTAMPNTZ", "TO_TIMESTAMP_NTZ"),
    ("TIMESTAMP_LTZ", "TO_TIMESTAMP_LTZ"),
    ("TIMESTAMPLTZ", "TO_TIMESTAMP_LTZ"),
    ("TIMESTAMP_TZ", "TO_TIMESTAMP_TZ"),
    ("TIMESTAMPTZ", "TO_TIMESTAMP_TZ"),
    ("DATETIME", "TO_TIMESTAMP_NTZ"),
];

/// Type names Snowflake takes before a string only without parameters, each with its function.
pub const SNOWFLAKE_UNPARAMETERIZED_LITERAL_TYPES: &[(&str, &str)] =
    &[("TIMESTAMP", "TO_TIMESTAMP"), ("TIME", "TO_TIME")];
