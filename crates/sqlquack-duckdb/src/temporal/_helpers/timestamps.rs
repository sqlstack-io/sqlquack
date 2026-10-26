//! Integer epoch arithmetic on DuckDB timestamps.

use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::{DUCKDB_FRACTIONAL_PRECISION, NANOSECOND_DIGITS};

/// The digits of the DuckDB timestamp that holds a naive Snowflake timestamp of `precision`.
pub(crate) fn stored_digits(precision: u8) -> u8 {
    match precision <= DUCKDB_FRACTIONAL_PRECISION {
        true => DUCKDB_FRACTIONAL_PRECISION,
        false => NANOSECOND_DIGITS,
    }
}

/// The timestamp's units since the epoch at its stored digits, as a BIGINT.
pub(crate) fn epoch_units(sql: &str, digits: u8) -> String {
    match digits == NANOSECOND_DIGITS {
        true => format!("epoch_ns({sql})"),
        false => format!("epoch_us({sql})"),
    }
}

/// `units` cut down to `to` digits, rounding toward negative infinity, in units of `to` digits.
pub(crate) fn floor_units(units: &str, from: u8, to: u8) -> String {
    let divisor = 10_i64.pow(u32::from(from.saturating_sub(to)));
    match divisor {
        1 => units.to_owned(),
        _ => format!(
            "({units} // {divisor} - (CASE WHEN {units} % {divisor} < 0 THEN 1 ELSE 0 END))"
        ),
    }
}

/// The naive DuckDB timestamp for `units` at `digits`, held at `stored` digits.
pub(crate) fn from_units(units: &str, digits: u8, stored: u8) -> String {
    let scale = 10_i64.pow(u32::from(stored - digits));
    let scaled = match scale {
        1 => units.to_owned(),
        _ => format!("{units} * {scale}"),
    };
    match stored == NANOSECOND_DIGITS {
        true => format!("make_timestamp_ns({scaled})"),
        false => format!("make_timestamp({scaled})"),
    }
}

/// A DATE as is, or a naive timestamp at `precision` truncated to microseconds.
pub(crate) fn microsecond_base(sql: &str, source: SnowflakeType) -> String {
    match source {
        SnowflakeType::TimestampNtz { precision } if precision > DUCKDB_FRACTIONAL_PRECISION => {
            let units = floor_units(&epoch_units(sql, NANOSECOND_DIGITS), NANOSECOND_DIGITS, 6);
            from_units(
                &units,
                DUCKDB_FRACTIONAL_PRECISION,
                DUCKDB_FRACTIONAL_PRECISION,
            )
        }
        _ => sql.to_owned(),
    }
}

/// A microsecond result back at the source's precision, with a nanosecond source's last digits.
pub(crate) fn with_source_nanos(micros: &str, source_sql: &str, source: SnowflakeType) -> String {
    match source {
        SnowflakeType::TimestampNtz { precision } if precision > DUCKDB_FRACTIONAL_PRECISION => {
            let nanos = epoch_units(source_sql, NANOSECOND_DIGITS);
            format!(
                "make_timestamp_ns(epoch_us({micros}) * 1000 + ((({nanos}) % 1000) + 1000) % 1000)"
            )
        }
        _ => format!("CAST({micros} AS TIMESTAMP)"),
    }
}

/// `value` divided by `divisor`, rounding toward negative infinity.
pub(crate) fn floor_divide(value: &str, divisor: i64) -> String {
    match divisor {
        1 => value.to_owned(),
        _ => format!(
            "({value} // {divisor} - (CASE WHEN {value} % {divisor} < 0 THEN 1 ELSE 0 END))"
        ),
    }
}
