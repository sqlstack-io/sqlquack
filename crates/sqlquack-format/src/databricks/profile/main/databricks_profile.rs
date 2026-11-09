//! `databricks_profile`: why Databricks SQL has no format profile yet.

use crate::databricks::profile::constants::UNSUPPORTED_FAILURE;
use crate::dialects::models::FormatProfile;

/// Databricks SQL's format profile: none until it has a format gate, so its SQL is refused.
pub(crate) fn databricks_profile() -> Result<FormatProfile, &'static str> {
    Err(UNSUPPORTED_FAILURE)
}
