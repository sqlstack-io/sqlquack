//! `databricks_rules`: Databricks SQL's lineage rules, as the dialect registry hands them out.

use crate::databricks::profile::models::DatabricksRules;
use crate::lineage::types::DialectRules;

/// Databricks SQL's lineage rules.
pub(crate) fn databricks_rules() -> &'static dyn DialectRules {
    &DatabricksRules
}
