//! `recorded_zone`: a time zone name column checked against the names Snowflake accepted.

use crate::macros::constants::TIME_ZONE_V1;
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;

/// The zone name `sql` as it is when NULL or recorded; raises time_zone_spelling otherwise.
pub fn recorded_zone(sql: &str) -> String {
    helper_call(TIME_ZONE_V1, &[UNIDENTIFIED_SITE, sql])
}
