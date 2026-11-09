//! Snowflake type spellings compared by the type they denote.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::dialects::main::canonical_type::canonical_type;
use crate::snowflake::profile::constants::SNOWFLAKE_LONGEST_VARCHAR;
use crate::snowflake::typing::main::cast_target::cast_target;
use crate::snowflake::typing::models::{CastTarget, SnowflakeType};

/// Whether two Snowflake type spellings denote one type: synonyms, the default length and precision.
pub(crate) fn same_snowflake_type(left: &str, right: &str) -> bool {
    match (snowflake_type(left), snowflake_type(right)) {
        (Some(left), Some(right)) => left == right,
        _ => canonical_type(left) == canonical_type(right),
    }
}

/// What a Snowflake type spelling denotes: its type, longest text, and which semi-structured type.
fn snowflake_type(spelling: &str) -> Option<(SnowflakeType, u32, String)> {
    let target: CastTarget = match cast_target(spelling, &ProvenanceLog::default()) {
        Ok(target) => target,
        Err(_unknown) => return None,
    };
    let semi_structured: String = match target.data_type {
        SnowflakeType::SemiStructured => canonical_type(spelling),
        _ => String::new(),
    };
    let longest: u32 = match target.data_type {
        SnowflakeType::Varchar => target.max_length.unwrap_or(SNOWFLAKE_LONGEST_VARCHAR),
        _ => 0,
    };
    Some((target.data_type, longest, semi_structured))
}
