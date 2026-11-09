//! `sum`: the type of SUM.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::number;
use crate::typing::constants::{MAX_PRECISION, SUM, SUM_EXTRA_DIGITS};
use crate::typing::models::SnowflakeType;

/// SUM of a NUMBER gains 12 leading digits, capped at 38; SUM of FLOAT is FLOAT.
pub fn sum(argument: SnowflakeType, log: &ProvenanceLog) -> Result<SnowflakeType, String> {
    log.typing(SUM);
    match argument {
        SnowflakeType::Number { precision, scale } => Ok(number(
            (u32::from(precision) + u32::from(SUM_EXTRA_DIGITS)).min(u32::from(MAX_PRECISION)),
            u32::from(scale),
        )),
        SnowflakeType::Float => Ok(SnowflakeType::Float),
        other => Err(format!("SUM of {other} has no proven type")),
    }
}
