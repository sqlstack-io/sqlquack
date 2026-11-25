//! `exact_float_moments`: a group's count, exact FLOAT sum and exact sum of squares, the values listed once.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::floats::_helpers::sums::{listed_values, values_sum};
use crate::floats::constants::{EXACT_FLOAT_SUM, FLOAT_SUM_ORDER};
use crate::raises::main::raised::raised;

/// The group's non-NULL values as a lambda parameter.
const VALUES: &str = "float_moment_values";

/// `body(count, total, squares)` over `value`'s group, `value` written once; each sum raises as `exact_float_sum`'s.
pub fn exact_float_moments(
    value: &str,
    over: &str,
    body: impl FnOnce(&str, &str, &str) -> String,
    log: &ProvenanceLog,
) -> String {
    log.building_block(EXACT_FLOAT_SUM);
    let raise = raised(&format!(
        "{FLOAT_SUM_ORDER}: a FLOAT sum whose value depends on the order of addition"
    ));
    let squares = format!(
        "list_transform({VALUES}, lambda float_moment_x: float_moment_x * float_moment_x)"
    );
    format!(
        "list_transform([{}], lambda {VALUES}: {})[1]",
        listed_values(value, over),
        body(
            &format!("len({VALUES})"),
            &values_sum(VALUES, false, &raise),
            &values_sum(&squares, false, &raise)
        )
    )
}
