//! `number_literal`: the type of a numeric literal.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::_helpers::numbers::number;
use crate::snowflake::typing::constants::{MAX_PRECISION, NUMBER_LITERAL};
use crate::snowflake::typing::main::literal_digits::literal_digits;
use crate::snowflake::typing::models::SnowflakeType;

/// The type of a numeric literal: its plain digits less trailing fractional zeros.
pub fn number_literal(text: &str, log: &ProvenanceLog) -> Result<SnowflakeType, String> {
    log.typing(NUMBER_LITERAL);
    let refused = || format!("numeric literal `{text}` has no proven Snowflake type");
    let digits = literal_digits(text)?;
    let (whole, fraction) = digits.split_once('.').unwrap_or((&digits, ""));
    let kept_fraction = fraction.trim_end_matches('0');
    let leading = whole.len();
    let precision = u32::try_from(leading + kept_fraction.len()).map_err(|_| refused())?;
    let scale = u32::try_from(kept_fraction.len()).map_err(|_| refused())?;
    if precision > u32::from(MAX_PRECISION) {
        return Err(refused());
    }
    Ok(number(precision.max(1), scale))
}
