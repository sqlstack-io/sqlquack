//! `postgres_rules`: PostgreSQL's lineage rules, as the dialect registry hands them out.

use crate::lineage::types::DialectRules;
use crate::postgres::profile::models::PostgresRules;

/// PostgreSQL's lineage rules.
pub(crate) fn postgres_rules() -> &'static dyn DialectRules {
    &PostgresRules
}
