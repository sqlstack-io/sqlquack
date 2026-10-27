//! `json_type_name`: Snowflake's TYPEOF of a VARIANT.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::semi_structured::constants::{JSON_TYPE_NAME, VARIANT_TYPED_VALUES};
use crate::variants::_helpers::objects::{bound, typed_text};

/// INTEGER, DECIMAL, NULL_VALUE and so on; with `typed`, strings a typed value may be raise.
pub fn json_type_name(sql: &str, typed: bool, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TYPE_NAME);
    let body = "(CASE json_type(j) WHEN 'NULL' THEN 'NULL_VALUE' WHEN 'BIGINT' THEN 'INTEGER' \
         WHEN 'UBIGINT' THEN 'INTEGER' WHEN 'DOUBLE' THEN CASE WHEN contains(CAST(j AS \
         VARCHAR), '.') THEN 'DECIMAL' ELSE 'INTEGER' END WHEN 'VARCHAR' THEN 'VARCHAR' \
         WHEN 'BOOLEAN' THEN 'BOOLEAN' WHEN 'OBJECT' THEN 'OBJECT' WHEN 'ARRAY' THEN 'ARRAY' END)";
    let body = match typed {
        true => format!(
            "(CASE WHEN {} THEN CAST({} AS VARCHAR) ELSE {body} END)",
            typed_text("j"),
            raised(&format!(
                "{VARIANT_TYPED_VALUES}: TYPEOF of a string that may be a typed value in Snowflake"
            ))
        ),
        false => body.to_owned(),
    };
    bound(sql, "j", &body)
}
