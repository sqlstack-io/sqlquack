//! `ranking`: the type of the ranking window functions.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::_helpers::numbers::number;
use crate::snowflake::typing::constants::{COUNT_PRECISION, RANKING};
use crate::snowflake::typing::models::SnowflakeType;

/// ROW_NUMBER, RANK and DENSE_RANK are `NUMBER(18,0)`.
pub fn ranking(log: &ProvenanceLog) -> SnowflakeType {
    log.typing(RANKING);
    number(u32::from(COUNT_PRECISION), 0)
}
