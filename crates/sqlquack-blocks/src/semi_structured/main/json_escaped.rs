//! `json_escaped`: DuckDB's JSON text with Snowflake's escapes.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::writing::escaped;
use crate::semi_structured::constants::JSON_ESCAPED;

/// The JSON text `sql` with DEL as `\u007f` and `\u` hex digits in lower case, as Snowflake writes.
pub fn json_escaped(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ESCAPED);
    escaped(sql)
}
