//! `evidence_policy`: the evidence policy compiled into SQLQuack, parsed once.

use std::sync::OnceLock;

use crate::policy::_helpers::parsing::parse_policy;
use crate::policy::constants::EVIDENCE_TEXT;
use crate::policy::models::Policy;

/// The compiled-in evidence policy, which the crate's tests parse; empty only if it were invalid.
pub fn evidence_policy() -> &'static Policy {
    static PARSED: OnceLock<Policy> = OnceLock::new();
    PARSED.get_or_init(|| parse_policy(EVIDENCE_TEXT).unwrap_or_default())
}
