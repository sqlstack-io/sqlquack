//! `json_scalar`: a VARIANT cast to NUMBER, FLOAT, BOOLEAN, DATE or TIMESTAMP_NTZ.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::macros::main::scalar_call::scalar_call;
use crate::semi_structured::constants::JSON_SCALAR;

/// Strings read as text, numbers as text or epoch seconds, JSON null as NULL; others raise.
pub fn json_scalar(
    sql: &str,
    target: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(JSON_SCALAR);
    scalar_call(sql, target, log)
}
