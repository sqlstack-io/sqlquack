//! `double_written`: a FLOAT held at the root of a VARIANT, as Snowflake writes it as JSON.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::guards::bound;
use crate::held::constants::{HELD_DOUBLE_FORMAT, VARIANT_WRITTEN};

/// A finite double `variant_double_leaf` held at a VARIANT's root, written as `%.15e`.
pub fn double_written(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_WRITTEN);
    bound(
        &format!("CAST(CAST({sql} AS JSON) AS DOUBLE)"),
        "written_double",
        &format!("printf('{HELD_DOUBLE_FORMAT}', written_double)"),
    )
}
