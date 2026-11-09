//! `average`: the type of AVG.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::_helpers::numbers::{capped, division_scale};
use crate::snowflake::typing::constants::{
    AVERAGE, AVERAGE_EXTRA_DIGITS, DIVISION_EXTRA_SCALE, SCALE_CAP,
};
use crate::snowflake::typing::models::SnowflakeType;

/// AVG of a NUMBER: division's scale, and 12 more leading digits; AVG of FLOAT is FLOAT.
pub fn average(argument: SnowflakeType, log: &ProvenanceLog) -> Result<SnowflakeType, String> {
    log.typing(AVERAGE);
    match argument {
        SnowflakeType::Number { precision, scale }
            if u32::from(scale) + u32::from(DIVISION_EXTRA_SCALE) <= u32::from(SCALE_CAP) =>
        {
            let leading = u32::from(precision - scale) + u32::from(AVERAGE_EXTRA_DIGITS);
            Ok(capped(leading, division_scale(u32::from(scale))))
        }
        SnowflakeType::Float => Ok(SnowflakeType::Float),
        other => Err(format!("AVG of {other} has no proven type")),
    }
}
