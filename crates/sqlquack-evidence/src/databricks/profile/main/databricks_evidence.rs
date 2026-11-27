//! `databricks_evidence`: the Databricks SQL evidence profile, for the registry and the importer.

use crate::databricks::profile::models::DatabricksEvidence;

/// The Databricks SQL evidence profile.
pub fn databricks_evidence() -> &'static DatabricksEvidence {
    &DatabricksEvidence
}
