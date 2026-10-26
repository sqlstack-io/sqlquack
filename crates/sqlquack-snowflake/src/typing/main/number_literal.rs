//! `number_literal`: the type of a numeric literal.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::number;
use crate::typing::constants::{MAX_PRECISION, NUMBER_LITERAL};
use crate::typing::models::SnowflakeType;

/// The type of a numeric literal: digits as written, trailing fractional zeros dropped.
pub fn number_literal(text: &str, log: &ProvenanceLog) -> Result<SnowflakeType, String> {
    log.typing(NUMBER_LITERAL);
    let refused = || format!("numeric literal `{text}` has no proven Snowflake type");
    if text.is_empty()
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Err(refused());
    }
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if fraction.contains('.') || (whole.len() > 1 && whole.starts_with('0')) {
        return Err(refused());
    }
    let kept_fraction = fraction.trim_end_matches('0');
    if !fraction.is_empty() && kept_fraction.is_empty() {
        return Err(refused());
    }
    let leading = whole.trim_start_matches('0').len();
    let leading = match (leading, kept_fraction.is_empty()) {
        (0, true) => 1,
        (digits, _) => digits,
    };
    let precision = u32::try_from(leading + kept_fraction.len()).map_err(|_| refused())?;
    let scale = u32::try_from(kept_fraction.len()).map_err(|_| refused())?;
    if precision > u32::from(MAX_PRECISION) {
        return Err(refused());
    }
    Ok(number(precision.max(1), scale))
}
