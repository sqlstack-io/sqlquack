//! Why SQL in a dialect built without its feature has no fingerprint.

use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::models::FingerprintDialect;

/// The failure for SQL in `dialect`, whose feature the crate was built without.
pub(crate) fn not_built(dialect: FingerprintDialect) -> FingerprintError {
    FingerprintError::Tokenize(format!(
        "SQLQuack's fingerprints were built without the feature for {dialect:?}"
    ))
}
