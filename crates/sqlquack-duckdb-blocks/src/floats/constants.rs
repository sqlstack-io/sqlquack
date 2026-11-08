//! FLOAT building block identifiers and limits.

/// A FLOAT as Snowflake's text: `duckdb.float_to_text`.
pub const FLOAT_TO_TEXT: &str = "duckdb.float_to_text";
/// A FLOAT rounded exactly to a NUMBER with a fraction: `duckdb.float_to_decimal`.
pub const FLOAT_TO_DECIMAL: &str = "duckdb.float_to_decimal";
/// The known divergence for FLOAT values whose text no recording shows.
pub const FLOAT_TEXT: &str = "float_text";
/// The smallest positive normal double; smaller non-zero magnitudes are subnormal.
pub const SMALLEST_NORMAL_DOUBLE: &str = "2.2250738585072014e-308";
/// Doubles of at least this magnitude are whole numbers (2^53).
pub const EXACT_DOUBLE_INTEGER: &str = "9007199254740992";
/// Below this magnitude a FLOAT rounds to zero at every supported scale.
pub const VANISHING_DOUBLE: &str = "1e-19";
/// The power of two a double is scaled by, less its binary exponent, to make it a whole number.
pub const SIGNIFICAND_SHIFT: i32 = 60;
/// The scales recordings show a FLOAT rounded to.
pub const FLOAT_DECIMAL_SCALES: &[u8] = &[2, 3, 4, 5, 6, 8, 10];
/// The divergence of a FLOAT rounded to more significant digits than recordings show.
pub const FLOAT_DECIMAL_DIGITS: &str = "float_decimal_digits";
/// The most significant digits of a rounded FLOAT recordings show: its scaled value stays below.
pub const FLOAT_DECIMAL_UNITS_LIMIT: u128 = 10_000_000_000;
/// The fewest significant digits Snowflake writes a FLOAT's text with: below 10, and below 1.
pub const FLOAT_TEXT_MIN_DIGITS: u8 = 10;
/// The most significant digits Snowflake writes a FLOAT's text with: from 1e5 on.
pub const FLOAT_TEXT_MAX_DIGITS: u8 = 15;
/// The digits beyond its whole digits Snowflake writes a FLOAT's text with, between those bounds.
pub const FLOAT_TEXT_FRACTION_DIGITS: u8 = 9;
/// The divergence for FLOAT sums whose value depends on the order of addition.
pub const FLOAT_SUM_ORDER: &str = "float_sum_order";
/// SUM of FLOAT, exact in any order or raising: `duckdb.exact_float_sum`.
pub const EXACT_FLOAT_SUM: &str = "duckdb.exact_float_sum";
/// Powers of two whose multiples, summing in magnitude to at most 2^52 over them, add exactly.
pub const EXACT_SUM_SCALES: &[u32] = &[0, 16, 32];
/// The largest magnitude sum, in units of the scale, that every order adds exactly, with margin.
pub const EXACT_SUM_UNITS: u64 = 1 << 52;
/// The bound on the rounding of a partial sum of up to 2^32 values, over their exact sum.
pub const DOMINANCE_MARGIN: &str = "1.000001";
/// LN, zero or less raising: `duckdb.natural_logarithm`.
pub const NATURAL_LOGARITHM: &str = "duckdb.natural_logarithm";
/// POWER(10, n) of a whole exponent: `duckdb.power_of_ten`.
pub const POWER_OF_TEN: &str = "duckdb.power_of_ten";
/// EXP, beyond the double range raising: `duckdb.exponential`.
pub const EXPONENTIAL: &str = "duckdb.exponential";
/// SQRT, a negative number raising: `duckdb.square_root`.
pub const SQUARE_ROOT: &str = "duckdb.square_root";
