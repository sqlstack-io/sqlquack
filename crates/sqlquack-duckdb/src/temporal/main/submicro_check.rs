//! `submicro_check`: a nanosecond timestamp that raises when it has a sub-microsecond part.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{SUBMICRO_CHECK, SUBMICRO_MESSAGE};
use crate::blocks::main::raised::raised;

/// The nanosecond timestamp, or an error when its sub-microsecond part is not zero.
pub fn submicro_check(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(SUBMICRO_CHECK);
    format!(
        "(CASE WHEN epoch_ns({sql}) % 1000 <> 0 THEN {} ELSE {sql} END)",
        raised(SUBMICRO_MESSAGE)
    )
}
