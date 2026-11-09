//! `framed_average`: the type of AVG over an explicit window frame.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::capped;
use crate::typing::constants::{
    AVERAGE_EXTRA_DIGITS, FRAMED_AVERAGE, FRAMED_AVERAGE_EXTRA_SCALE, SCALE_CAP,
};
use crate::typing::models::SnowflakeType;

/// AVG over an explicit frame or unordered partition: 3 more places, 12 more digits, as recorded.
pub fn framed_average(
    argument: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(FRAMED_AVERAGE);
    match argument {
        SnowflakeType::Number { precision, scale }
            if scale + FRAMED_AVERAGE_EXTRA_SCALE <= SCALE_CAP =>
        {
            let leading = u32::from(precision - scale) + u32::from(AVERAGE_EXTRA_DIGITS);
            Ok(capped(
                leading,
                u32::from(scale + FRAMED_AVERAGE_EXTRA_SCALE),
            ))
        }
        SnowflakeType::Float => Ok(SnowflakeType::Float),
        other => Err(format!(
            "AVG of {other} over a window frame has no proven type"
        )),
    }
}
