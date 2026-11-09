//! `confirm_assumption`: conversion options trusting the caller's word for a named assumption.

use sqlquack_convert::snowflake::conversion::models::{AllowedDivergence, ConversionOptions};
use sqlquack_policy::policy::models::Policy;

/// `options` relying on the policy's assumption `id` in any mode: the caller confirmed it.
pub fn confirm_assumption(
    mut options: ConversionOptions,
    policy: &Policy,
    id: &str,
) -> ConversionOptions {
    if let Some(divergence) = policy.divergences.get(id) {
        options.divergences.insert(
            id.to_owned(),
            AllowedDivergence {
                constructs: divergence.constructs.clone(),
                reason: divergence.reason.clone(),
                assumption: divergence.assumption,
            },
        );
    }
    options
}
