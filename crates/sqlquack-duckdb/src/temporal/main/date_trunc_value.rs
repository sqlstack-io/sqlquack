//! `date_trunc_value`: a DATE or naive timestamp truncated to a unit, keeping its type.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::DATE_TRUNC;
use crate::blocks::main::cast::cast;
use crate::temporal::_helpers::timestamps::microsecond_base;

/// The value truncated to the unit named by its canonical Snowflake name; weeks start on Monday.
pub fn date_trunc_value(
    sql: &str,
    unit: &str,
    source: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(DATE_TRUNC);
    let clock = matches!(source, SnowflakeType::TimestampNtz { .. });
    let unit = match unit {
        "YEAR" | "QUARTER" | "MONTH" | "WEEK" | "DAY" => unit.to_ascii_lowercase(),
        "HOUR" | "MINUTE" | "SECOND" if clock => unit.to_ascii_lowercase(),
        other => {
            return Err(format!(
                "DATE_TRUNC to {other} of {source} has no proven conversion"
            ));
        }
    };
    let base = microsecond_base(sql, source);
    cast(&format!("date_trunc('{unit}', {base})"), source, log)
}
