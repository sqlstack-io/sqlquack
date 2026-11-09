//! Test case types for duckdb fingerprints.

use crate::fingerprint::models::FingerprintDialect;

/// SQL that must fail with a message containing `expected_fragment`.
pub(crate) struct FailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) sql: &'static str,
    pub(crate) expected_fragment: &'static str,
}

/// SQL whose fingerprint must stay the recorded digest.
pub(crate) struct GoldenTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) sql: &'static str,
    pub(crate) expected_digest: &'static str,
}

/// Pathological SQL and whether its fingerprint finishes within the nesting limit.
pub(crate) struct NestingTestCase {
    pub(crate) description: &'static str,
    pub(crate) dialect: FingerprintDialect,
    pub(crate) sql: String,
    pub(crate) expected_within_limit: bool,
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
