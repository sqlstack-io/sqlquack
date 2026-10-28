//! Literal date and timestamp formats.

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

/// A field a timestamp format element reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimestampField {
    /// Four-digit year.
    Year,
    /// Month, one or two digits.
    Month,
    /// Day of the month, one or two digits.
    Day,
    /// Hour 0-23, one or two digits.
    Hour24,
    /// Hour 1-12, one or two digits, with a meridiem.
    Hour12,
    /// Minute, one or two digits.
    Minute,
    /// Second, one or two digits.
    Second,
    /// AM or PM in any case.
    Meridiem,
}

/// A literal timestamp format read as a whole-text pattern with one group per field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimestampFormat {
    /// The whole-text regular expression.
    pub pattern: String,
    /// Each group's field, in group order.
    pub fields: Vec<TimestampField>,
}
