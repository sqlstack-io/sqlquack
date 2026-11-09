//! `exact_float_sum`: SUM of FLOAT where every order of addition gives one value, else a raise.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::INLINE_ARGUMENT_LIMIT;
use crate::floats::_helpers::sums::{aggregated_sum, listed_sum};
use crate::floats::constants::{EXACT_FLOAT_SUM, FLOAT_SUM_ORDER};
use crate::raises::main::raised::raised;

/// `sum(distinct value) over` where every order of addition agrees exactly; otherwise raising.
pub fn exact_float_sum(value: &str, distinct: &str, over: &str, log: &ProvenanceLog) -> String {
    log.building_block(EXACT_FLOAT_SUM);
    let raise = raised(&format!(
        "{FLOAT_SUM_ORDER}: a FLOAT sum whose value depends on the order of addition"
    ));
    match value.len() > INLINE_ARGUMENT_LIMIT {
        true => listed_sum(value, !distinct.is_empty(), over, &raise),
        false => aggregated_sum(value, distinct, over, &raise),
    }
}
