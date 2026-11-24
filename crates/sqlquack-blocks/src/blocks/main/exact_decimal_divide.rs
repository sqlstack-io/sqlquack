//! `exact_decimal_divide`: exact decimal division at the result scale, rounded or truncated.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::_helpers::division::{
    long_quotient, null_checks, scaled, short_quotient, unscaled_literal,
};
use crate::blocks::constants::EXACT_DECIMAL_DIVIDE;

/// Exact division, rounded or `truncated`, by a literal `constant` divisor if given; may it raise?
/// Unless `checked`, the caller already makes the quotient NULL where an operand is NULL or the
/// divisor zero, so the short form adds no checks of its own.
pub fn exact_decimal_divide(
    left: (&str, u8, u8),
    right: (&str, u8, u8),
    (result, truncated, constant, checked): (SnowflakeType, bool, Option<&str>, bool),
    log: &ProvenanceLog,
) -> Result<(String, bool), String> {
    log.building_block(EXACT_DECIMAL_DIVIDE);
    let SnowflakeType::Number {
        scale: result_scale,
        ..
    } = result
    else {
        return Err(format!(
            "exact division needs a NUMBER result, not {result}"
        ));
    };
    let constant = constant
        .and_then(|text| unscaled_literal(text, right.2))
        .filter(|divisor| *divisor != 0);
    let checks = null_checks(left.0, right.0, constant.is_some());
    let rounding = (result_scale, truncated);
    if let Some((quotient, places)) = short_quotient((left, right), rounding, constant) {
        let value = scaled(&quotient, (result, places), log)?;
        return Ok((
            match constant.is_none() && checked {
                false => value,
                true => format!("(CASE {checks} ELSE {value} END)"),
            },
            false,
        ));
    }
    let (quotient, guarded) = long_quotient((left, right), rounding, constant)?;
    Ok((scaled(&quotient, (result, result_scale), log)?, guarded))
}
