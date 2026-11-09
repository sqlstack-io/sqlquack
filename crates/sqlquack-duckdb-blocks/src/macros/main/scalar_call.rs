//! `scalar_call`: the helper macro call casting a VARIANT to NUMBER, FLOAT, BOOLEAN, DATE or TIMESTAMP_NTZ.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::constants::TEXT_TO_NUMBER;
use crate::blocks::main::column_type::column_type;
use crate::macros::_helpers::scalars::{largest_number, scalar_case, scalar_macro};
use crate::macros::constants::VARIANT_NUMBER_V1;
use crate::macros::main::helper_call::helper_call;
use crate::raises::constants::UNIDENTIFIED_SITE;

/// The call casting the VARIANT `sql` to `target`, logging the blocks its macro's body uses.
pub fn scalar_call(
    sql: &str,
    target: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    if let SnowflakeType::Number { precision, scale } = target {
        log.building_block(TEXT_TO_NUMBER);
        let duckdb = column_type(target, log)?;
        return Ok(helper_call(
            VARIANT_NUMBER_V1,
            &[
                UNIDENTIFIED_SITE,
                sql,
                &format!("CAST(NULL AS {duckdb})"),
                &format!("CAST('{}' AS {duckdb})", largest_number(precision, scale)),
            ],
        ));
    }
    let name = scalar_macro(target)
        .ok_or_else(|| format!("casting a VARIANT to {target} has no proven conversion"))?;
    scalar_case(target, log)?;
    Ok(helper_call(name, &[UNIDENTIFIED_SITE, sql]))
}
