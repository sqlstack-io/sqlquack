//! `unresolved`: the refusal of a name that does not resolve.

use crate::refusals::models::{Refusal, RefusalCategory};

/// A name does not resolve.
pub fn unresolved(reason: String) -> Refusal {
    Refusal {
        construct: None,
        location: None,
        category: RefusalCategory::UnresolvedName,
        reason,
    }
}
