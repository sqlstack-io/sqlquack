//! `unify`: one type for several branches.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::_helpers::numbers::unify_pair;
use crate::snowflake::typing::constants::{UNIFY, VARIANT_COMMON_TYPE};
use crate::snowflake::typing::models::SnowflakeType;

/// One type for values from several branches, or a refusal when no unification is proven.
pub fn unify(types: &[SnowflakeType], log: &ProvenanceLog) -> Result<SnowflakeType, String> {
    log.typing(UNIFY);
    let mut unified = SnowflakeType::Null;
    for data_type in types {
        unified = match (unified, *data_type) {
            (SnowflakeType::SemiStructured, SnowflakeType::SemiStructured) => {
                drop(log.rule(VARIANT_COMMON_TYPE));
                SnowflakeType::SemiStructured
            }
            (left, right) => unify_pair(left, right)?,
        };
    }
    Ok(unified)
}
