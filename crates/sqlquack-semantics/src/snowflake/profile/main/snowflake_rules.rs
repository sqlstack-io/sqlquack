//! `snowflake_rules`: Snowflake's lineage rules, as the dialect registry hands them out.

use crate::lineage::types::DialectRules;
use crate::snowflake::profile::models::SnowflakeRules;

/// Snowflake's lineage rules.
pub(crate) fn snowflake_rules() -> &'static dyn DialectRules {
    &SnowflakeRules
}
