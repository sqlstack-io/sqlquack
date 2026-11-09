//! `bigquery_rules`: BigQuery's lineage rules, as the dialect registry hands them out.

use crate::bigquery::profile::models::BigQueryRules;
use crate::lineage::types::DialectRules;

/// BigQuery's lineage rules.
pub(crate) fn bigquery_rules() -> &'static dyn DialectRules {
    &BigQueryRules
}
