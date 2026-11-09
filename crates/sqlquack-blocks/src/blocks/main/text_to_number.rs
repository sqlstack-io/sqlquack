//! `text_to_number`: text to NUMBER, accepting only spellings Snowflake is recorded to read.

use sqlquack_core::failures::constants::NUMBER_NOT_RECOGNIZED;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::_helpers::text::{
    conversion_failure, once_bound, overflow_failure, space_trimmed,
};
use crate::blocks::_helpers::types_map::duckdb_type;
use crate::blocks::constants::{
    BARE_EXPONENT_PATTERN, DECIMAL_TEXT_PATTERN, MANTISSA_FREE_PATTERN, TEXT_TO_NUMBER,
};
use crate::blocks::models::TextFailure;

/// Text to `result`, rounding half away from zero; unrecorded spellings and overflow fail per `failure`.
pub fn text_to_number(
    sql: &str,
    result: SnowflakeType,
    failure: TextFailure,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(TEXT_TO_NUMBER);
    let SnowflakeType::Number { precision, scale } = result else {
        return Err(format!(
            "text_to_number needs a NUMBER target, not {result}"
        ));
    };
    let target = duckdb_type(result, log)?;
    Ok(once_bound(sql, "number_text", |sql| {
        read_number(sql, (&target, precision, scale), failure)
    }))
}

/// The CASE reading the text `sql` as the DuckDB `target` of NUMBER(`precision`, `scale`).
fn read_number(
    sql: &str,
    (target, precision, scale): (&str, u8, u8),
    failure: TextFailure,
) -> String {
    let trimmed = space_trimmed(sql);
    let overflow = format!("CAST({} AS {target})", overflow_failure(failure));
    let whole = "9".repeat(usize::from(precision - scale));
    let largest = match scale {
        0 => whole,
        _ => format!("{whole}.{}", "9".repeat(usize::from(scale))),
    };
    let parsed = format!("TRY_CAST({trimmed} AS {target})");
    let beyond = format!("abs({parsed}) > CAST('{largest}' AS {target})");
    format!(
        "(CASE WHEN {sql} IS NULL THEN NULL \
         WHEN regexp_matches({trimmed}, '[eE]') AND {beyond} THEN {overflow} \
         WHEN regexp_full_match({trimmed}, '{DECIMAL_TEXT_PATTERN}') \
         THEN coalesce(TRY_CAST({trimmed} AS {target}), {overflow}) \
         WHEN regexp_full_match({trimmed}, '{BARE_EXPONENT_PATTERN}') \
         THEN coalesce(TRY_CAST(left({trimmed}, length({trimmed}) - 1) AS {target}), {overflow}) \
         WHEN regexp_full_match({trimmed}, '{MANTISSA_FREE_PATTERN}') THEN CAST(0 AS {target}) \
         ELSE CAST({} AS {target}) END)",
        conversion_failure(
            failure,
            (NUMBER_NOT_RECOGNIZED, "Numeric value is not recognized")
        )
    )
}
