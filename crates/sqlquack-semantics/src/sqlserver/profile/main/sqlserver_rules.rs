//! `sqlserver_rules`: SQL Server's lineage rules, as the dialect registry hands them out.

use crate::lineage::types::DialectRules;
use crate::sqlserver::profile::models::SqlServerRules;

/// SQL Server's lineage rules.
pub(crate) fn sqlserver_rules() -> &'static dyn DialectRules {
    &SqlServerRules
}
