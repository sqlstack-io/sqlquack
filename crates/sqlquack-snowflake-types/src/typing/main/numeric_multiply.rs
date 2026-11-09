//! `numeric_multiply`: the type of `*`.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::{arithmetic_operands, capped};
use crate::typing::constants::{NUMERIC_MULTIPLY, SCALE_CAP};
use crate::typing::models::SnowflakeType;

/// `*`: leading digits add; scales add up to 12, or to the larger operand scale above 12.
pub fn numeric_multiply(
    left: SnowflakeType,
    right: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(NUMERIC_MULTIPLY);
    Ok(match arithmetic_operands(left, right, "*")? {
        None => SnowflakeType::Float,
        Some(((leading_left, scale_left), (leading_right, scale_right))) => {
            let ceiling = scale_left.max(scale_right).max(u32::from(SCALE_CAP));
            capped(
                leading_left + leading_right,
                (scale_left + scale_right).min(ceiling),
            )
        }
    })
}
