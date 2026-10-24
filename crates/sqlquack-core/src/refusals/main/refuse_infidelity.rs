//! `refuse_infidelity`: the refusal for a statement whose print-back changes its tree.

use crate::refusals::models::{Refusal, RefusalCategory};
use crate::reparse::models::FidelityOutcome;

/// Refuses a statement unless printing it back in its own dialect reproduced the same tree.
pub fn refuse_infidelity(outcome: &FidelityOutcome) -> Option<Refusal> {
    let reason = match outcome {
        FidelityOutcome::Identical => return None,
        FidelityOutcome::Canonicalised { difference } => format!(
            "printing back canonicalises the tree at {}, and no recorded proof covers that \
             canonicalisation",
            difference.path
        ),
        FidelityOutcome::Unstable { difference } => format!(
            "printing back does not reach a stable tree; it changes at {}",
            difference.path
        ),
        FidelityOutcome::PrintFailed { message } => {
            format!("the parsed tree does not print back: {message}")
        }
        FidelityOutcome::ReparseFailed { message } => {
            format!("the printed SQL does not parse again: {message}")
        }
    };
    Some(Refusal {
        construct: None,
        location: None,
        category: RefusalCategory::ParseFidelity,
        reason,
    })
}
