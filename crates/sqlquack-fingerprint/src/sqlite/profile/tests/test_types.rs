//! Test case types for sqlite fingerprints.

use crate::fingerprint::models::FingerprintDialect;

/// Two queries and whether they must share a fingerprint.
pub(crate) struct StabilityTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) first: &'static str,
    pub(crate) second: &'static str,
    pub(crate) expected_same: bool,
}
