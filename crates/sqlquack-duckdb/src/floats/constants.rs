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
/// The decimal places a FLOAT below one was recorded written to.
pub const FLOAT_TEXT_PLACES: u8 = 15;
