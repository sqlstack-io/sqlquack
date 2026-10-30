//! `missing`: the refusal of a construct no rule converts.

use crate::classification::models::Construct;
use crate::refusals::models::{Refusal, RefusalCategory};

/// No rule converts the construct.
pub fn missing(construct: Construct, reason: String) -> Refusal {
    Refusal {
        construct: Some(construct),
        location: None,
        category: RefusalCategory::MissingRule,
        reason,
    }
}
