//! The replay cache's keys: digests of everything a converted case's judgement is computed from.

use std::fmt::Debug;

use sha2::{Digest, Sha256};
use sqlquack_policy::policy::models::Policy;

use crate::cases::models::LoadedCase;
use crate::constants::REPLAY_CACHE_FORMAT;
use crate::results::models::ResultMetadata;
use crate::verify::types::FailureModes;

/// The digest every key shares: the cache format, the caller's scope, the policy and failure modes.
pub(crate) fn replay_context(scope: &str, policy: &Policy, failure_modes: &FailureModes) -> String {
    digest(format!("{REPLAY_CACHE_FORMAT}\n{scope}\n{policy:?}\n{failure_modes:?}").as_bytes())
}

/// The key of one converted case's judgement.
pub(crate) fn judgement_key(
    context: &str,
    (case, metadata): (&LoadedCase, &ResultMetadata),
    conversion: &dyn Debug,
) -> String {
    digest(format!("{context}\n{case:?}\n{metadata:?}\n{conversion:?}").as_bytes())
}

/// The SHA-256 of `text`, as lower-case hex.
fn digest(text: &[u8]) -> String {
    format!("{:x}", Sha256::digest(text))
}
