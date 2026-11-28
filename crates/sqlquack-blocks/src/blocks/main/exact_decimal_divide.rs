//! `exact_decimal_divide`: exact decimal division at the result scale, rounded or truncated.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::_helpers::division::{
    long_quotient, null_checks, scaled, short_quotient, unscaled_literal,
};
use crate::blocks::constants::EXACT_DECIMAL_DIVIDE;
use crate::blocks::models::Divisor;

/// Exact division, rounded or `truncated`, by the right operand as `divisor`; may it raise?
pub fn exact_decimal_divide(
    left: (&str, u8, u8),
    right: (&str, u8, u8),
    (result, truncated, divisor): (SnowflakeType, bool, Divisor<'_>),
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
    let constant = match divisor {
        Divisor::Literal(text) => unscaled_literal(text, right.2).filter(|value| *value != 0),
        Divisor::Checked | Divisor::Guarded => None,
    };
    let checks = null_checks(left.0, right.0, constant.is_some());
    let rounding = (result_scale, truncated);
    if let Some(quotient) = short_quotient((left, right), rounding, constant) {
        let value = scaled(&quotient, (result, result_scale), log)?;
        return Ok((
            match constant.is_none() && divisor != Divisor::Guarded {
                false => value,
                true => format!("(CASE {checks} ELSE {value} END)"),
            },
            false,
        ));
    }
    let (quotient, guarded) = long_quotient((left, right), rounding, constant)?;
    Ok((scaled(&quotient, (result, result_scale), log)?, guarded))
}
