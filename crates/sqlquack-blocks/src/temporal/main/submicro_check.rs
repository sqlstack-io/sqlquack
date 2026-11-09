//! `submicro_check`: a nanosecond timestamp that raises when it has a sub-microsecond part.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{SUBMICRO_CHECK, SUBMICRO_MESSAGE};
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::operands::once;

/// The nanosecond timestamp, or an error when its sub-microsecond part is not zero.
pub fn submicro_check(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(SUBMICRO_CHECK);
    let raise = raised(SUBMICRO_MESSAGE);
    once(sql, "submicro_value", |value| {
        format!("(CASE WHEN epoch_ns({value}) % 1000 <> 0 THEN {raise} ELSE {value} END)")
    })
}
