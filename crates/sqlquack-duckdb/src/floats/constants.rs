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
pub const FLOAT_DECIMAL_SCALES: &[u8] = &[2, 6, 8];
/// The divergence of a FLOAT rounded to more significant digits than recordings show.
pub const FLOAT_DECIMAL_DIGITS: &str = "float_decimal_digits";
/// The most significant digits of a rounded FLOAT recordings show: its scaled value stays below.
pub const FLOAT_DECIMAL_UNITS_LIMIT: u128 = 1_000_000_000;
/// Significant digits Snowflake writes a FLOAT's text with near one and below normal doubles.
pub const FLOAT_TEXT_SHORT_DIGITS: u8 = 10;
/// Significant digits Snowflake writes a FLOAT's text with in the recorded wider magnitudes.
pub const FLOAT_TEXT_LONG_DIGITS: u8 = 15;
/// Magnitude ranges, [from, to), whose text Snowflake was recorded writing with ten digits.
pub const FLOAT_TEXT_SHORT_RANGES: &[(&str, &str)] =
    &[("0", "2.2250738585072014e-308"), ("0.1", "10")];
/// Magnitude ranges, [from, to), whose text Snowflake was recorded writing with fifteen digits.
pub const FLOAT_TEXT_LONG_RANGES: &[(&str, &str)] =
    &[("1e5", "1e6"), ("1e8", "1e9"), ("1e15", "inf")];
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
