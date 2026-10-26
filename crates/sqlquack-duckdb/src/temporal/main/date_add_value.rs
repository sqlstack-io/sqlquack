//! `date_add_value`: a DATE or naive timestamp moved by whole calendar or clock units.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::DATE_ADD;
use crate::temporal::_helpers::timestamps::{microsecond_base, with_source_nanos};

/// The value plus `amount` units; month arithmetic keeps the day, clamped to the month's end.
pub fn date_add_value(
    sql: &str,
    (unit, amount): (&str, &str),
    source: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(DATE_ADD);
    let clock = matches!(source, SnowflakeType::TimestampNtz { .. });
    let interval = match unit {
        "YEAR" => format!("to_years({amount})"),
        "QUARTER" => format!("to_months(3 * {amount})"),
        "MONTH" => format!("to_months({amount})"),
        "WEEK" => format!("to_days(7 * {amount})"),
        "DAY" => format!("to_days({amount})"),
        "HOUR" if clock => format!("to_hours({amount})"),
        "MINUTE" if clock => format!("to_minutes({amount})"),
        "SECOND" if clock => format!("to_seconds({amount})"),
        other => {
            return Err(format!(
                "DATEADD of {other} to {source} has no proven conversion"
            ));
        }
    };
    let base = microsecond_base(sql, source);
    let moved = format!("({base} + {interval})");
    Ok(match source {
        SnowflakeType::Date => format!("CAST({moved} AS DATE)"),
        _ => with_source_nanos(&moved, sql, source),
    })
}
