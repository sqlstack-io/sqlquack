//! `builtin_catalogue`: the catalogue SQLQuack ships.

use crate::catalogue::constants::NOT_EXPRESSIBLE;
use crate::catalogue::models::RuleCatalogue;

/// Every verified rule and declared refusal.
pub fn builtin_catalogue() -> RuleCatalogue {
    RuleCatalogue {
        not_expressible: NOT_EXPRESSIBLE.to_vec(),
    }
}
