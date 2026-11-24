//! `variant_written`: a VARIANT-held value as Snowflake writes it as JSON.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::held::_helpers::writing::held_written_call;
use crate::held::constants::VARIANT_WRITTEN;

/// The VARIANT-held `sql` as Snowflake's JSON text: keys sorted, doubles as `%.15e`.
pub fn variant_written(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_WRITTEN);
    held_written_call(sql)
}
