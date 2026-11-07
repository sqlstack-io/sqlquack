//! `date_diff_value`: the unit boundaries crossed from one DATE or naive timestamp to another.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::DATE_DIFF;
use crate::temporal::_helpers::timestamps::{epoch_units, floor_divide, stored_digits};
use crate::temporal::constants::MIDNIGHT_PRECISION;
use crate::temporal::main::truncate_timestamp_to_date::truncate_timestamp_to_date;

/// Boundaries of the unit named by its canonical Snowflake name crossed from `start` to `end`.
pub fn date_diff_value(
    (start, end): (&str, &str),
    unit: &str,
    (start_type, end_type): (SnowflakeType, SnowflakeType),
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(DATE_DIFF);
    let date = |sql: &str, source: SnowflakeType| match source {
        SnowflakeType::TimestampNtz { precision } => {
            truncate_timestamp_to_date(sql, precision, log)
        }
        _ => sql.to_owned(),
    };
    let (start_date, end_date) = (date(start, start_type), date(end, end_type));
    let calendar = |count: &dyn Fn(&str) -> String| {
        Ok(format!("({} - {})", count(&end_date), count(&start_date)))
    };
    let seconds_per_unit: i64 = match unit {
        "YEAR" => return calendar(&|day| format!("year({day})")),
        "QUARTER" => return calendar(&|day| format!("(year({day}) * 4 + quarter({day}))")),
        "MONTH" => return calendar(&|day| format!("(year({day}) * 12 + month({day}))")),
        "WEEK" => {
            return Ok(format!(
                "(date_diff('day', date_trunc('week', {start_date}), date_trunc('week', {end_date})) // 7)"
            ));
        }
        "DAY" => return Ok(format!("date_diff('day', {start_date}, {end_date})")),
        "HOUR" => 3600,
        "MINUTE" => 60,
        "SECOND" => 1,
        _ => 0,
    };
    let units = |sql: &str, source: SnowflakeType| -> Result<String, String> {
        let (sql, source) = match source {
            SnowflakeType::Date => (
                format!("CAST({sql} AS TIMESTAMP)"),
                SnowflakeType::TimestampNtz {
                    precision: MIDNIGHT_PRECISION,
                },
            ),
            other => (sql.to_owned(), other),
        };
        let sql = sql.as_str();
        let SnowflakeType::TimestampNtz { precision } = source else {
            return Err(format!(
                "DATEDIFF in {unit} of {source} has no proven conversion"
            ));
        };
        let digits = stored_digits(precision);
        let per_second = 10_i64.pow(u32::from(digits));
        let divisor = match (seconds_per_unit, unit) {
            (0, "MILLISECOND") => per_second / 1000,
            (0, "MICROSECOND") => per_second / 1_000_000,
            (0, _) => return Err(format!("DATEDIFF in {unit} has no proven conversion")),
            (seconds, _) => seconds * per_second,
        };
        Ok(floor_divide(&epoch_units(sql, digits), divisor))
    };
    Ok(format!(
        "({} - {})",
        units(end, end_type)?,
        units(start, start_type)?
    ))
}
