//! `sqlite_rules`: SQLite's lineage rules, as the dialect registry hands them out.

use crate::lineage::types::DialectRules;
use crate::sqlite::profile::models::SqliteRules;

/// SQLite's lineage rules.
pub(crate) fn sqlite_rules() -> &'static dyn DialectRules {
    &SqliteRules
}
