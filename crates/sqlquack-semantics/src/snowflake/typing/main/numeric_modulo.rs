//! `numeric_modulo`: the type of `%` and MOD.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::_helpers::numbers::{arithmetic_operands, capped};
use crate::snowflake::typing::constants::{MODULO_MIN_LEADING_DIGITS, NUMERIC_MODULO};
use crate::snowflake::typing::models::SnowflakeType;

/// `%`: the larger leading digits, at least two as recorded, and the larger scale.
pub fn numeric_modulo(
    left: SnowflakeType,
    right: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(NUMERIC_MODULO);
    Ok(match arithmetic_operands(left, right, "%")? {
        None => SnowflakeType::Float,
        Some(((leading_left, scale_left), (leading_right, scale_right))) => {
            let leading = leading_left
                .max(leading_right)
                .max(MODULO_MIN_LEADING_DIGITS);
            capped(leading, scale_left.max(scale_right))
        }
    })
}
