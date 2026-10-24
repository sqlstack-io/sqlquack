//! Warehouse column descriptions as Snowflake type names.

use crate::results::models::WarehouseColumn;

pub(crate) fn snowflake_type(column: &WarehouseColumn) -> String {
    let precision = column.precision.unwrap_or_default();
    let scale = column.scale.unwrap_or_default();
    let size = column.internal_size.unwrap_or_default();
    match column.type_name.as_str() {
        "FIXED" => format!("NUMBER({precision},{scale})"),
        "REAL" => "FLOAT".to_owned(),
        "TEXT" => format!("VARCHAR({size})"),
        "BINARY" => format!("BINARY({size})"),
        "TIME" | "TIMESTAMP_NTZ" | "TIMESTAMP_LTZ" | "TIMESTAMP_TZ" => {
            format!("{}({scale})", column.type_name)
        }
        other => other.to_owned(),
    }
}
