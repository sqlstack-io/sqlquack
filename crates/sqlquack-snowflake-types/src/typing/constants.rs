//! Typing function identifiers, as conversion provenance records them, and type limits.

use crate::typing::models::SnowflakeType;

/// Snowflake's largest NUMBER precision.
pub const MAX_PRECISION: u8 = 38;
/// The scale `*` and `/` stop raising at, unless an operand already has more.
pub const SCALE_CAP: u8 = 12;
/// The digits `/` and AVG add to the dividend's scale.
pub const DIVISION_EXTRA_SCALE: u8 = 6;
/// The digits AVG over an explicit window frame adds to its argument's scale, as recorded.
pub const FRAMED_AVERAGE_EXTRA_SCALE: u8 = 3;
/// The leading digits SUM adds to its argument.
pub const SUM_EXTRA_DIGITS: u8 = 12;
/// The leading digits AVG adds to its argument.
pub const AVERAGE_EXTRA_DIGITS: u8 = 12;
/// Precision of COUNT and of the ranking window functions.
pub const COUNT_PRECISION: u8 = 18;
/// Precision of timestamps and times without a declared precision.
pub const DEFAULT_FRACTIONAL_PRECISION: u8 = 9;

/// `NUMBER(38,0)`, which integer type names denote.
pub const NUMBER_38_0: SnowflakeType = SnowflakeType::Number {
    precision: MAX_PRECISION,
    scale: 0,
};

/// Type names and casts: `snowflake.cast_target`.
pub const CAST_TARGET: &str = "snowflake.cast_target";
/// Numeric literals: `snowflake.number_literal`.
pub const NUMBER_LITERAL: &str = "snowflake.number_literal";
/// `+` and `-`: `snowflake.numeric_add_subtract`.
pub const NUMERIC_ADD_SUBTRACT: &str = "snowflake.numeric_add_subtract";
/// `%` and MOD: `snowflake.numeric_modulo`.
pub const NUMERIC_MODULO: &str = "snowflake.numeric_modulo";
/// The fewest digits before the point Snowflake gives `%`, whatever its operands' own.
pub const MODULO_MIN_LEADING_DIGITS: u32 = 2;
/// `*`: `snowflake.numeric_multiply`.
pub const NUMERIC_MULTIPLY: &str = "snowflake.numeric_multiply";
/// `/`: `snowflake.numeric_divide`.
pub const NUMERIC_DIVIDE: &str = "snowflake.numeric_divide";
/// One result type for CASE, COALESCE, IFF, NVL, NULLIF and set operations: `snowflake.unify`.
pub const UNIFY: &str = "snowflake.unify";
/// The rule a conversion relies on when semi-structured branches unify as VARIANT.
pub const VARIANT_COMMON_TYPE: &str = "rule.variant_common_type";
/// Operands a comparison accepts without an implicit conversion: `snowflake.comparison`.
pub const COMPARISON: &str = "snowflake.comparison";
/// COUNT: `snowflake.count`.
pub const COUNT: &str = "snowflake.count";
/// SUM: `snowflake.sum`.
pub const SUM: &str = "snowflake.sum";
/// AVG: `snowflake.average`.
pub const AVERAGE: &str = "snowflake.average";
/// AVG over an explicit window frame: `snowflake.framed_average`.
pub const FRAMED_AVERAGE: &str = "snowflake.framed_average";
/// ROW_NUMBER, RANK and DENSE_RANK: `snowflake.ranking`.
pub const RANKING: &str = "snowflake.ranking";

/// Base names of Snowflake's structured types, which take element, field or key types.
pub const STRUCTURED_TYPES: &[&str] = &["ARRAY", "OBJECT", "MAP"];
/// Why a structured type is refused until rules for it have evidence.
pub const STRUCTURED_UNSUPPORTED: &str = "is not supported yet";
/// The recorded type of DATEDIFF between two TIMEs.
pub const TIME_DIFF_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 9,
    scale: 0,
};
/// The type of DATEDIFF in microseconds between two TIMEs, whose values reach 86,400,000,000.
pub const TIME_DIFF_MICROSECONDS_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 18,
    scale: 0,
};
/// DATEDIFF's units and the precision of their recorded NUMBER(p,0) type.
pub const DATEDIFF_PRECISIONS: &[(&str, u8)] = &[
    ("YEAR", 9),
    ("QUARTER", 9),
    ("MONTH", 9),
    ("WEEK", 9),
    ("DAY", 9),
    ("HOUR", 9),
    ("MINUTE", 18),
    ("SECOND", 18),
    ("MILLISECOND", 38),
    ("MICROSECOND", 38),
];
/// Snowflake's documented abbreviations of date and time parts, with their full names.
pub const UNIT_ALIASES: &[(&str, &str)] = &[
    ("Y", "YEAR"),
    ("YY", "YEAR"),
    ("YYY", "YEAR"),
    ("YYYY", "YEAR"),
    ("YR", "YEAR"),
    ("YEARS", "YEAR"),
    ("YRS", "YEAR"),
    ("Q", "QUARTER"),
    ("QTR", "QUARTER"),
    ("QTRS", "QUARTER"),
    ("QUARTERS", "QUARTER"),
    ("MM", "MONTH"),
    ("MON", "MONTH"),
    ("MONS", "MONTH"),
    ("MONTHS", "MONTH"),
    ("W", "WEEK"),
    ("WK", "WEEK"),
    ("WEEKOFYEAR", "WEEK"),
    ("WOY", "WEEK"),
    ("WY", "WEEK"),
    ("D", "DAY"),
    ("DD", "DAY"),
    ("DAYS", "DAY"),
    ("DAYOFMONTH", "DAY"),
    ("DOW", "DAYOFWEEK"),
    ("DW", "DAYOFWEEK"),
    ("WEEKDAY", "DAYOFWEEK"),
    ("DOW_ISO", "DAYOFWEEKISO"),
    ("WEEKDAY_ISO", "DAYOFWEEKISO"),
    ("DOY", "DAYOFYEAR"),
    ("DY", "DAYOFYEAR"),
    ("YEARDAY", "DAYOFYEAR"),
    ("H", "HOUR"),
    ("HH", "HOUR"),
    ("HR", "HOUR"),
    ("HOURS", "HOUR"),
    ("HRS", "HOUR"),
    ("M", "MINUTE"),
    ("MI", "MINUTE"),
    ("MIN", "MINUTE"),
    ("MINUTES", "MINUTE"),
    ("MINS", "MINUTE"),
    ("S", "SECOND"),
    ("SEC", "SECOND"),
    ("SECONDS", "SECOND"),
    ("SECS", "SECOND"),
    ("MS", "MILLISECOND"),
    ("MSEC", "MILLISECOND"),
    ("MILLISECONDS", "MILLISECOND"),
    ("US", "MICROSECOND"),
    ("USEC", "MICROSECOND"),
    ("MICROSECONDS", "MICROSECOND"),
    ("NS", "NANOSECOND"),
    ("NSEC", "NANOSECOND"),
    ("NANOSEC", "NANOSECOND"),
    ("NANOSECONDS", "NANOSECOND"),
    ("EPOCH", "EPOCH_SECOND"),
    ("EPOCH_SECONDS", "EPOCH_SECOND"),
    ("EPOCH_MILLISECONDS", "EPOCH_MILLISECOND"),
];
/// The most digits a numeric literal's exponent is read with.
pub const MAX_EXPONENT_DIGITS: usize = 2;
/// The furthest a numeric literal's exponent moves its decimal point, in either direction.
pub const MAX_POINT_SHIFT: usize = 64;
/// The recorded type of COUNT_IF.
pub const COUNT_IF_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 13,
    scale: 0,
};

/// Text read beside other branches' types: `snowflake.text_reading`.
pub const TEXT_READING: &str = "snowflake.text_reading";
/// The NUMBER Snowflake reads text as beside numbers in COALESCE, NVL, IFF and CASE.
pub const TEXT_AS_NUMBER: SnowflakeType = SnowflakeType::Number {
    precision: 18,
    scale: 5,
};
