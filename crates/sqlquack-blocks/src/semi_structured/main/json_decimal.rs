//! `json_decimal`: a NUMBER with a scale put in a VARIANT, as the JSON number Snowflake writes.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::bound;
use crate::semi_structured::constants::JSON_DECIMAL;
use crate::semi_structured::main::json_guarded::json_guarded;

/// Trailing fraction zeros dropped and a whole value without a point, as JSON; guarded as parsed.
pub fn json_decimal(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_DECIMAL);
    let text = bound(
        sql,
        "d",
        "(CASE WHEN d = trunc(d) THEN CAST(CAST(trunc(d) AS DECIMAL(38,0)) AS VARCHAR) \
         ELSE rtrim(CAST(d AS VARCHAR), '0') END)",
    );
    json_guarded(&text, false, log)
}
