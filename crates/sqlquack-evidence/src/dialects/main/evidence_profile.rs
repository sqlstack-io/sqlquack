//! `evidence_profile`: the registry of each recording warehouse's evidence profile.

use sqlquack_policy::policy::models::EvidenceDialect;

#[cfg(feature = "databricks")]
use crate::databricks::profile::main::databricks_evidence::databricks_evidence;
#[cfg(not(feature = "databricks"))]
use crate::dialects::constants::NOT_COMPILED;
use crate::dialects::constants::SHARED_EVIDENCE;
use crate::dialects::types::EvidenceProfile;

/// The evidence profile of `dialect`, or why this build cannot read its evidence.
pub fn evidence_profile(dialect: EvidenceDialect) -> Result<&'static dyn EvidenceProfile, String> {
    match dialect {
        #[cfg(feature = "databricks")]
        EvidenceDialect::Databricks => Ok(databricks_evidence()),
        #[cfg(not(feature = "databricks"))]
        EvidenceDialect::Databricks => Err(format!("{NOT_COMPILED} {}", dialect.name())),
        _ => Ok(&SHARED_EVIDENCE),
    }
}
