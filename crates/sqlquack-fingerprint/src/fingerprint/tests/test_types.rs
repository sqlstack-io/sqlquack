//! Test case types for fingerprints.

use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::models::FingerprintDialect;

/// A dialect name and the dialect it resolves to.
pub(crate) struct DialectNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_dialect: Result<FingerprintDialect, FingerprintError>,
}

/// SQL whose fingerprint must stay the recorded digest.
pub(crate) struct GoldenTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) sql: &'static str,
    pub(crate) expected_digest: &'static str,
}

/// Two queries and whether they must share a fingerprint.
pub(crate) struct StabilityTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) first: &'static str,
    pub(crate) second: &'static str,
    pub(crate) expected_same: bool,
}

/// SQL and its exact canonical token stream.
pub(crate) struct TokensTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) sql: &'static str,
    pub(crate) expected_tokens: &'static [&'static str],
}
