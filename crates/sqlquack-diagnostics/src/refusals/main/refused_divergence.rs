//! `refused_divergence`: the refusal of a construct whose conversion needs a refused divergence.

use crate::classification::models::Construct;
use crate::refusals::constants::{NEEDS_DIVERGENCE, REFUSED_IN_MODE};
use crate::refusals::models::{Refusal, RefusalCategory};

/// The construct converts only by relying on `divergence`, which the policy refuses in this mode.
pub fn refused_divergence(construct: Construct, divergence: &str) -> Refusal {
    Refusal {
        reason: format!(
            "{}{NEEDS_DIVERGENCE}{divergence}{REFUSED_IN_MODE}",
            construct.key()
        ),
        construct: Some(construct),
        location: None,
        category: RefusalCategory::DivergenceRefused,
    }
}
