//! `exact_decimal_divide`: exact decimal division at the result scale, rounded or truncated.

use sqlquack_diagnostics::failures::constants::DIVISION_BY_ZERO;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::_helpers::text::tagged_error;
use crate::blocks::constants::{EXACT_DECIMAL_DIVIDE, MAX_DECIMAL_PRECISION};
use crate::blocks::main::cast::cast;
use crate::raises::main::raised::raised;

/// Exact division, rounded half away from zero or `truncated`, and whether it may raise for range.
pub fn exact_decimal_divide(
    left: (&str, u8, u8),
    right: (&str, u8, u8),
    (result, truncated): (SnowflakeType, bool),
    log: &ProvenanceLog,
) -> Result<(String, bool), String> {
    log.building_block(EXACT_DECIMAL_DIVIDE);
    let SnowflakeType::Number {
        scale: result_scale,
        ..
    } = result
    else {
        return Err(format!(
            "exact division needs a NUMBER result, not {result}"
        ));
    };
    let (left_sql, _, left_scale) = left;
    let (right_sql, right_precision, right_scale) = right;
    let shift = result_scale + right_scale - left_scale.min(result_scale + right_scale);
    if shift >= MAX_DECIMAL_PRECISION {
        return Err(format!(
            "a scale shift of {shift} leaves no room in DuckDB's 128-bit integers"
        ));
    }
    let guarded = u32::from(right_precision) + u32::from(shift) > u32::from(MAX_DECIMAL_PRECISION);
    let unscaled = |sql: &str, scale: u8| {
        let decimal = format!("CAST({sql} AS DECIMAL(38,{scale}))");
        let power = "0".repeat(usize::from(scale));
        format!(
            "(CAST(trunc({decimal}) AS HUGEINT) * CAST('1{power}' AS HUGEINT) + \
             CAST(({decimal} - trunc({decimal})) * CAST('1{power}' AS DECIMAL(38,0)) AS HUGEINT))"
        )
    };
    let dividend = unscaled(left_sql, left_scale);
    let divisor = unscaled(right_sql, right_scale);
    let power = format!("CAST('1{}' AS HUGEINT)", "0".repeat(usize::from(shift)));
    let remainder = format!("(({dividend} % {divisor}) * {power})");
    let left_over = format!("abs({remainder} % {divisor})");
    let rounding = match truncated {
        true => "0".to_owned(),
        false => format!(
            "(CASE WHEN {left_over} >= abs({divisor}) - {left_over} \
             THEN sign({remainder}) * sign({divisor}) ELSE 0 END)"
        ),
    };
    let rounded =
        format!("(({dividend} // {divisor}) * {power} + ({remainder} // {divisor}) + {rounding})");
    let unit = match result_scale {
        0 => "1".to_owned(),
        scale => format!("0.{}1", "0".repeat(usize::from(scale) - 1)),
    };
    let value = cast(
        &format!("CAST({rounded} AS DECIMAL(38,0)) * CAST('{unit}' AS DECIMAL(38,{result_scale}))"),
        result,
        log,
    )?;
    let guard = match guarded {
        true => format!(
            " WHEN abs({divisor}) >= CAST('1{}' AS HUGEINT) THEN {}",
            "0".repeat(usize::from(MAX_DECIMAL_PRECISION - shift)),
            raised("numeric_range: the exact quotient needs more than 128 bits")
        ),
        false => String::new(),
    };
    Ok((
        format!(
            "(CASE WHEN {left_sql} IS NULL OR {right_sql} IS NULL THEN NULL \
             WHEN {right_sql} = 0 THEN {}{guard} ELSE {value} END)",
            tagged_error(DIVISION_BY_ZERO, "Division by zero")
        ),
        guarded,
    ))
}
