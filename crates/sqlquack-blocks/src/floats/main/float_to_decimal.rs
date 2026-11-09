//! `float_to_decimal`: a FLOAT rounded exactly to a NUMBER with a fraction.

use sqlquack_diagnostics::failures::constants::NUMBER_OUT_OF_RANGE;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use sqlquack_diagnostics::failures::main::tagged_message::tagged_message;

use crate::blocks::constants::NUMBER_RANGE_MESSAGE;
use crate::blocks::main::column_type::column_type;
use crate::floats::_helpers::values::bound;
use crate::floats::constants::{
    EXACT_DOUBLE_INTEGER, FLOAT_DECIMAL_DIGITS, FLOAT_DECIMAL_SCALES, FLOAT_DECIMAL_UNITS_LIMIT,
    FLOAT_TO_DECIMAL, SIGNIFICAND_SHIFT, VANISHING_DOUBLE,
};
use crate::raises::main::raised::raised;

/// The double's exact value rounded half away from zero at the scale; NaN or overflow raise 100046.
pub fn float_to_decimal(
    value: &str,
    result: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(FLOAT_TO_DECIMAL);
    let SnowflakeType::Number { precision, scale } = result else {
        return Err(format!(
            "float_to_decimal needs a NUMBER target, not {result}"
        ));
    };
    if !FLOAT_DECIMAL_SCALES.contains(&scale) {
        return Err(format!(
            "a FLOAT rounded to scale {scale}, which no recording shows"
        ));
    }
    let target = column_type(result, log)?;
    let sql = "f";
    let magnitude = format!("abs({sql})");
    let unit = 10_u128.pow(u32::from(scale));
    let limit = 10_u128.pow(u32::from(precision));
    let shift = format!("({SIGNIFICAND_SHIFT} - CAST(floor(log2({magnitude})) AS INTEGER))");
    let significand = format!("CAST({magnitude} * pow(2.0, {shift}) AS HUGEINT)");
    let units = format!(
        "(CASE WHEN {magnitude} >= {EXACT_DOUBLE_INTEGER} \
         THEN CAST({magnitude} AS HUGEINT) * CAST({unit} AS HUGEINT) \
         WHEN {magnitude} < {VANISHING_DOUBLE} THEN CAST(0 AS HUGEINT) \
         ELSE (2 * {significand} * CAST({unit} AS HUGEINT) + (CAST(1 AS HUGEINT) << {shift})) \
         // (CAST(1 AS HUGEINT) << ({shift} + 1)) END)"
    );
    let overflow = format!(
        "CAST({} AS {target})",
        raised(&tagged_message(NUMBER_OUT_OF_RANGE, NUMBER_RANGE_MESSAGE))
    );
    let unproven = format!(
        "CAST({} AS {target})",
        raised(&format!(
            "{FLOAT_DECIMAL_DIGITS}: a FLOAT rounded to more significant digits than recordings show"
        ))
    );
    let body = format!(
        "(CASE WHEN {sql} IS NULL THEN NULL \
         WHEN isnan({sql}) OR isinf({sql}) OR {magnitude} >= 1e{whole} THEN {overflow} \
         WHEN {units} >= CAST({limit} AS HUGEINT) THEN {overflow} \
         WHEN {units} >= CAST({FLOAT_DECIMAL_UNITS_LIMIT} AS HUGEINT) THEN {unproven} \
         ELSE CAST(CAST(CASE WHEN {sql} < 0 THEN -{units} ELSE {units} END AS DECIMAL(38,0)) \
         * CAST('1e-{scale}' AS DECIMAL(38,{scale})) AS {target}) END)",
        whole = precision - scale
    );
    Ok(bound(value, sql, &body))
}
