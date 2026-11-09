//! Test case types for databricks fingerprints.

use crate::fingerprint::models::FingerprintDialect;

/// Two queries and whether they must share a fingerprint.
pub(crate) struct StabilityTestCase {
    pub(crate) description: &'static str,
    pub(crate) first: &'static str,
    pub(crate) second: &'static str,
    pub(crate) expected_same: bool,
}

/// SQL and the other dialect whose fingerprint its Databricks fingerprint must equal.
pub(crate) struct SameAsTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_same_as: FingerprintDialect,
}
