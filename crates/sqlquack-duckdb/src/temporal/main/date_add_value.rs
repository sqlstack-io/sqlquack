//! `date_add_value`: a DATE or naive timestamp moved by whole calendar or clock units.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::{
    DATE_ADD, DATE_ADD_DAY_LIMIT, DATE_ADD_MONTH_LIMIT, DUCKDB_FRACTIONAL_PRECISION,
    MICROSECOND_TIMESTAMP_LIMIT, TIMESTAMP_RANGE,
};
use crate::raises::main::raised::raised;
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
    let beyond = match source {
        SnowflakeType::TimestampNtz { precision } if precision > DUCKDB_FRACTIONAL_PRECISION => {
            raised(&format!(
                "{TIMESTAMP_RANGE}: a DATEADD result beyond the range DuckDB holds"
            ))
        }
        _ => raised("a DATEADD result beyond the range DuckDB holds, which no recording shows"),
    };
    let calendar = |function: &str, per: i64, limit: i64| {
        format!(
            "list_transform([CAST({amount} AS HUGEINT) * {per}], lambda date_add_steps: \
             CASE WHEN abs(date_add_steps) > {limit} THEN CAST({beyond} AS INTERVAL) \
             ELSE {function}(CAST(date_add_steps AS INTEGER)) END)[1]"
        )
    };
    let micros_per = match unit {
        "HOUR" if clock => Some(3_600_000_000_i64),
        "MINUTE" if clock => Some(60_000_000),
        "SECOND" if clock => Some(1_000_000),
        "MILLISECOND" if clock => Some(1_000),
        "MICROSECOND" if clock => Some(1),
        _ => None,
    };
    let base = microsecond_base(sql, source);
    let moved = match (unit, micros_per) {
        (_, Some(per)) => format!(
            "list_transform([CAST(epoch_us({base}) AS HUGEINT) + CAST({amount} AS HUGEINT) * {per}], \
             lambda date_add_micros: CASE WHEN abs(date_add_micros) > {MICROSECOND_TIMESTAMP_LIMIT} \
             THEN CAST({beyond} AS TIMESTAMP) \
             ELSE make_timestamp(CAST(date_add_micros AS BIGINT)) END)[1]"
        ),
        ("YEAR", None) => format!(
            "({base} + {})",
            calendar("to_months", 12, DATE_ADD_MONTH_LIMIT)
        ),
        ("QUARTER", None) => format!(
            "({base} + {})",
            calendar("to_months", 3, DATE_ADD_MONTH_LIMIT)
        ),
        ("MONTH", None) => format!(
            "({base} + {})",
            calendar("to_months", 1, DATE_ADD_MONTH_LIMIT)
        ),
        ("WEEK", None) => format!("({base} + {})", calendar("to_days", 7, DATE_ADD_DAY_LIMIT)),
        ("DAY", None) => format!("({base} + {})", calendar("to_days", 1, DATE_ADD_DAY_LIMIT)),
        (other, None) => {
            return Err(format!(
                "DATEADD of {other} to {source} has no proven conversion"
            ));
        }
    };
    Ok(match source {
        SnowflakeType::Date => format!("CAST({moved} AS DATE)"),
        _ => with_source_nanos(&moved, sql, source),
    })
}
