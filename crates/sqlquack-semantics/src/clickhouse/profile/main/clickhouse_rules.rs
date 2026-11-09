//! `clickhouse_rules`: ClickHouse's lineage rules, as the dialect registry hands them out.

use crate::clickhouse::profile::models::ClickHouseRules;
use crate::lineage::types::DialectRules;

/// ClickHouse's lineage rules.
pub(crate) fn clickhouse_rules() -> &'static dyn DialectRules {
    &ClickHouseRules
}
