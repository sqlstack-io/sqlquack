//! `outside_conditions`: the refusal of a construct outside its rule's operand types.

use crate::classification::models::Construct;
use crate::refusals::constants::{NEEDS_DIVERGENCE, REFUSED_IN_MODE};
use crate::refusals::models::{Refusal, RefusalCategory};

/// A rule covers the construct, but not these operands; a refused divergence's reason keeps its code.
pub fn outside_conditions(construct: Construct, reason: String) -> Refusal {
    let category = match reason.contains(NEEDS_DIVERGENCE) && reason.ends_with(REFUSED_IN_MODE) {
        true => RefusalCategory::DivergenceRefused,
        false => RefusalCategory::TypeCondition,
    };
    Refusal {
        construct: Some(construct),
        location: None,
        category,
        reason,
    }
}
