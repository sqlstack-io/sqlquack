//! `count`: the type of COUNT.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::typing::_helpers::numbers::number;
use crate::typing::constants::{COUNT, COUNT_PRECISION};
use crate::typing::models::SnowflakeType;

/// COUNT is `NUMBER(18,0)`.
pub fn count(log: &ProvenanceLog) -> SnowflakeType {
    log.typing(COUNT);
    number(u32::from(COUNT_PRECISION), 0)
}
