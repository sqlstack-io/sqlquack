//! `numeric_add_subtract`: the type of `+` and `-`.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::{arithmetic_operands, capped};
use crate::typing::constants::NUMERIC_ADD_SUBTRACT;
use crate::typing::models::SnowflakeType;

/// `+` and `-`: the larger scale, and one more leading digit than the larger operand.
pub fn numeric_add_subtract(
    left: SnowflakeType,
    right: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(NUMERIC_ADD_SUBTRACT);
    Ok(match arithmetic_operands(left, right, "+")? {
        None => SnowflakeType::Float,
        Some(((leading_left, scale_left), (leading_right, scale_right))) => capped(
            leading_left.max(leading_right) + 1,
            scale_left.max(scale_right),
        ),
    })
}
