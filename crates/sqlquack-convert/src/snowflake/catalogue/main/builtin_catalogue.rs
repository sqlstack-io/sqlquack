//! `builtin_catalogue`: the catalogue SQLQuack ships.

use crate::snowflake::catalogue::constants::NOT_EXPRESSIBLE;
use crate::snowflake::catalogue::models::RuleCatalogue;

/// Every verified rule and declared refusal.
pub fn builtin_catalogue() -> RuleCatalogue {
    RuleCatalogue {
        not_expressible: NOT_EXPRESSIBLE.to_vec(),
    }
}
