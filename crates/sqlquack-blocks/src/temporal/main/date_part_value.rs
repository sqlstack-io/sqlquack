//! `date_part_value`: one part of a DATE or naive timestamp, as a BIGINT.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::DATE_PART;
use crate::temporal::_helpers::timestamps::{
    epoch_units, floor_units, microsecond_base, stored_digits,
};

/// The part named by its canonical Snowflake name, with weeks as ISO weeks; zoned values by instant.
pub fn date_part_value(
    sql: &str,
    part: &str,
    source: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(DATE_PART);
    let base = microsecond_base(sql, source);
    let calendar = |function: &str| Ok(format!("{function}({base})"));
    let (instant, digits) = match source {
        SnowflakeType::TimestampNtz { precision } => {
            let digits = stored_digits(precision);
            (epoch_units(sql, digits), digits)
        }
        SnowflakeType::TimestampLtz { .. } | SnowflakeType::TimestampTz { .. } => {
            (format!("epoch_us({sql})"), 6)
        }
        _ => (format!("epoch_us(CAST({sql} AS TIMESTAMP))"), 6),
    };
    let clock = matches!(source, SnowflakeType::TimestampNtz { .. });
    match part {
        "YEAR" => calendar("year"),
        "QUARTER" => calendar("quarter"),
        "MONTH" => calendar("month"),
        "WEEK" | "WEEKISO" => calendar("week"),
        "DAY" => calendar("day"),
        "DAYOFWEEK" => calendar("dayofweek"),
        "DAYOFWEEKISO" => calendar("isodow"),
        "DAYOFYEAR" => calendar("dayofyear"),
        "YEAROFWEEK" | "YEAROFWEEKISO" => calendar("isoyear"),
        "HOUR" if clock => calendar("hour"),
        "MINUTE" if clock => calendar("minute"),
        "SECOND" if clock => calendar("second"),
        "NANOSECOND" if clock => {
            let modulus = 10_i64.pow(u32::from(digits));
            let scale = 10_i64.pow(u32::from(9 - digits));
            Ok(format!(
                "((({instant}) % {modulus} + {modulus}) % {modulus}) * {scale}"
            ))
        }
        "EPOCH_SECOND" => Ok(floor_units(&instant, digits, 0)),
        "EPOCH_MILLISECOND" => Ok(floor_units(&instant, digits, 3)),
        other => Err(format!(
            "DATE_PART {other} of {source} has no proven conversion"
        )),
    }
}
