//! `duckdb_rules`: DuckDB's lineage rules, as the dialect registry hands them out.

use crate::duckdb::profile::models::DuckDbRules;
use crate::lineage::types::DialectRules;

/// DuckDB's lineage rules.
pub(crate) fn duckdb_rules() -> &'static dyn DialectRules {
    &DuckDbRules
}
