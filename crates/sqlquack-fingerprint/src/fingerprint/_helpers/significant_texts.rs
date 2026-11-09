//! The significant token texts of SQL under a dialect's tokenizer.

use crate::dialects::main::profile::profile;
use crate::fingerprint::_helpers::not_built::not_built;
use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::models::FingerprintDialect;

/// The text of each significant token of `sql`, in order: no whitespace and no comments.
pub(crate) fn significant_texts(
    sql: &str,
    dialect: FingerprintDialect,
) -> Result<Vec<&str>, FingerprintError> {
    let profile = profile(dialect).ok_or_else(|| not_built(dialect))?;
    (profile.texts)(sql)
}
