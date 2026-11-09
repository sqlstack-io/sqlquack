//! Test case types for bigquery fingerprints.

use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::models::FingerprintDialect;

/// A dialect name and the dialect it resolves to.
pub(crate) struct DialectNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_dialect: Result<FingerprintDialect, FingerprintError>,
}

/// Two queries and whether they must share a fingerprint.
pub(crate) struct StabilityTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) first: &'static str,
    pub(crate) second: &'static str,
    pub(crate) expected_same: bool,
}
