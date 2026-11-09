//! `numeric_divide`: the type of `/`.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::{arithmetic_operands, capped, division_scale};
use crate::typing::constants::NUMERIC_DIVIDE;
use crate::typing::models::SnowflakeType;

/// `/`: the dividend's scale raised by 6 up to 12; leading digits gain the divisor's scale.
pub fn numeric_divide(
    left: SnowflakeType,
    right: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(NUMERIC_DIVIDE);
    Ok(match arithmetic_operands(left, right, "/")? {
        None => SnowflakeType::Float,
        Some(((leading_left, scale_left), (_, scale_right))) => {
            capped(leading_left + scale_right, division_scale(scale_left))
        }
    })
}
