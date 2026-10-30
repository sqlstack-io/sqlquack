//! Typing function identifiers, as conversion provenance records them, and type limits.

use crate::typing::models::SnowflakeType;

/// Snowflake's largest NUMBER precision.
pub const MAX_PRECISION: u8 = 38;
/// The scale `*` and `/` stop raising at, unless an operand already has more.
pub const SCALE_CAP: u8 = 12;
/// The digits `/` and AVG add to the dividend's scale.
pub const DIVISION_EXTRA_SCALE: u8 = 6;
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
/// The most digits a numeric literal's exponent is read with.
pub const MAX_EXPONENT_DIGITS: usize = 2;
/// The furthest a numeric literal's exponent moves its decimal point, in either direction.
pub const MAX_POINT_SHIFT: usize = 64;
/// The recorded type of COUNT_IF.
pub const COUNT_IF_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 13,
    scale: 0,
};
