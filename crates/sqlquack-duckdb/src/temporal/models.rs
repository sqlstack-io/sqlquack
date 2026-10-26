//! Literal date formats.

/// A literal Snowflake date format SQLQuack reads exactly: digits and separators, no padding rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateFormat {
    /// The Snowflake format text, such as `YYYYMMDD`.
    pub snowflake: &'static str,
    /// The whole-text regular expression with one group per field.
    pub pattern: &'static str,
    /// The year, month and day groups.
    pub groups: (u8, u8, u8),
    /// Whether the year has two digits, read from TWO_DIGIT_CENTURY_START.
    pub two_digit_year: bool,
}
