//! `refuse_value_divergences`: conversion options relying only on divergences that fail loudly.

use sqlquack_policy::policy::models::{DivergenceFailure, Policy};
use sqlquack_rules::conversion::models::ConversionOptions;

/// `options` without every divergence whose failure can be a different value rather than an error.
pub(crate) fn refuse_value_divergences(
    mut options: ConversionOptions,
    policy: &Policy,
) -> ConversionOptions {
    options.divergences.retain(|id, _| {
        policy
            .divergences
            .get(id)
            .is_some_and(|divergence| divergence.failure == DivergenceFailure::Loud)
    });
    options
}
