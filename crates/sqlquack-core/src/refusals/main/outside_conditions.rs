//! `outside_conditions`: the refusal of a construct outside its rule's operand types.

use crate::classification::models::Construct;
use crate::refusals::models::{Refusal, RefusalCategory};

/// A rule covers the construct, but not for these operand types.
pub fn outside_conditions(construct: Construct, reason: String) -> Refusal {
    Refusal {
        construct: Some(construct),
        location: None,
        category: RefusalCategory::TypeCondition,
        reason,
    }
}
