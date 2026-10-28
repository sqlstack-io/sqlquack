//! `numeric_modulo`: the type of `%` and MOD.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::{arithmetic_operands, capped};
use crate::typing::constants::NUMERIC_MODULO;
use crate::typing::models::SnowflakeType;

/// `%`: the larger operand's leading digits and the larger scale.
pub fn numeric_modulo(
    left: SnowflakeType,
    right: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(NUMERIC_MODULO);
    Ok(match arithmetic_operands(left, right, "%")? {
        None => SnowflakeType::Float,
        Some(((leading_left, scale_left), (leading_right, scale_right))) => {
            capped(leading_left.max(leading_right), scale_left.max(scale_right))
        }
    })
}
