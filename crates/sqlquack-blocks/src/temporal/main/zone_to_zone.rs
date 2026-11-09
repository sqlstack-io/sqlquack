//! `zone_to_zone`: a naive wall time in one zone as the wall time in another.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::ZONE_TO_ZONE;
use crate::temporal::_helpers::operands::once;
use crate::temporal::_helpers::timestamps::{microsecond_base, with_source_nanos};

/// The naive timestamp read in the zone `source` (repeated times early) as `target` wall time.
pub fn zone_to_zone(
    value: &str,
    [source, target]: [&str; 2],
    data_type: SnowflakeType,
    log: &ProvenanceLog,
) -> String {
    log.building_block(ZONE_TO_ZONE);
    once(value, "zone_wall", |sql| {
        once(source, "zone_name", |source| {
            converted(sql, [source, target], data_type)
        })
    })
}

/// The conversion of the plain operand `sql` from the plain zone `source`.
fn converted(sql: &str, [source, target]: [&str; 2], data_type: SnowflakeType) -> String {
    let base = microsecond_base(sql, data_type);
    let later = format!("timezone({source}, {base})");
    let day_before = format!("({later} - INTERVAL 1 DAY)");
    let earlier_offset =
        format!("(timezone({source}, {day_before}) - timezone('UTC', {day_before}))");
    let earlier = format!("timezone('UTC', {base} - {earlier_offset})");
    with_source_nanos(
        &format!(
            "timezone({target}, CASE WHEN timezone({source}, {earlier}) = {base} \
             THEN {earlier} ELSE {later} END)"
        ),
        sql,
        data_type,
    )
}
