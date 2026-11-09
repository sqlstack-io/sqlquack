//! `refuse_constructs`: one refusal for every construct no rule converts.

use crate::classification::models::Occurrence;
use crate::refusals::models::{Disposition, Refusal, RefusalCategory};
use crate::refusals::types::ConstructPolicy;

/// Refuses every occurrence the policy cannot convert, in order.
pub fn refuse_constructs(occurrences: &[Occurrence], policy: &dyn ConstructPolicy) -> Vec<Refusal> {
    occurrences
        .iter()
        .map(|occurrence| {
            let (category, reason) = match policy.disposition(&occurrence.construct) {
                Disposition::Missing => (
                    RefusalCategory::MissingRule,
                    format!("no verified rule converts {}", occurrence.construct),
                ),
                Disposition::NotExpressible { reason } => (RefusalCategory::NotExpressible, reason),
            };
            Refusal {
                construct: Some(occurrence.construct.clone()),
                location: occurrence.location,
                category,
                reason,
            }
        })
        .collect()
}
