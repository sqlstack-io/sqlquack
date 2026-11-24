//! `utc_to_zone`: a naive UTC wall time as the wall time in another zone.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::{
    DUCKDB_FRACTIONAL_PRECISION, NANOSECOND_TIMESTAMP_MICROS, TIMESTAMP_RANGE, UTC_TO_ZONE,
};
use crate::raises::main::raised::raised;
use crate::temporal::_helpers::operands::once;
use crate::temporal::_helpers::timestamps::{microsecond_base, with_source_nanos};

/// The naive timestamp read as UTC and shown as the wall time in `zone`, at its precision.
pub fn utc_to_zone(value: &str, zone: &str, source: SnowflakeType, log: &ProvenanceLog) -> String {
    log.building_block(UTC_TO_ZONE);
    once(value, "utc_wall", |sql| converted(sql, zone, source))
}

/// The conversion of the plain operand `sql`: a nanosecond one in a literal zone directly, raising
/// past the range whose whole microseconds the microsecond form holds. A zone column keeps the
/// microsecond form, as TRY would hide its NULLs and its unknown names' raise.
fn converted(sql: &str, zone: &str, source: SnowflakeType) -> String {
    match source {
        SnowflakeType::TimestampNtz { precision }
            if precision > DUCKDB_FRACTIONAL_PRECISION && is_text_literal(zone) =>
        {
            let lowest = -NANOSECOND_TIMESTAMP_MICROS * 1000;
            let highest = NANOSECOND_TIMESTAMP_MICROS * 1000 + 999;
            format!(
                "list_transform([TRY(timezone({zone}, timezone('UTC', {sql})))], lambda zone_wall: \
                 CASE WHEN zone_wall IS NULL AND {sql} IS NOT NULL \
                 OR NOT epoch_ns(zone_wall) BETWEEN {lowest} AND {highest} THEN {} \
                 ELSE zone_wall END)[1]",
                raised(&format!(
                    "{TIMESTAMP_RANGE}: a timestamp Snowflake holds outside the nanosecond range of \
                     DuckDB"
                ))
            )
        }
        _ => with_source_nanos(
            &format!(
                "timezone({zone}, timezone('UTC', {}))",
                microsecond_base(sql, source)
            ),
            sql,
            source,
        ),
    }
}

/// Whether `sql` is one string literal, as `string_literal` writes it.
fn is_text_literal(sql: &str) -> bool {
    sql.len() >= 2
        && sql.starts_with('\'')
        && sql.ends_with('\'')
        && !sql[1..sql.len() - 1].replace("''", "").contains('\'')
}
