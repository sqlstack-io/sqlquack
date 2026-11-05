//! `confirm_utc_session`: conversion options trusting the caller's word that sessions run in UTC.

use crate::conversion::constants::SESSION_TIMEZONE_UTC;
use crate::conversion::models::{AllowedDivergence, ConversionOptions};
use sqlquack_policy::policy::models::Policy;

/// `options` relying on `session_timezone_utc` in any mode: the caller confirmed UTC sessions.
pub fn confirm_utc_session(mut options: ConversionOptions, policy: &Policy) -> ConversionOptions {
    if let Some(divergence) = policy.divergences.get(SESSION_TIMEZONE_UTC) {
        options.divergences.insert(
            SESSION_TIMEZONE_UTC.to_owned(),
            AllowedDivergence {
                constructs: divergence.constructs.clone(),
                reason: divergence.reason.clone(),
                assumption: divergence.assumption,
            },
        );
    }
    options
}
