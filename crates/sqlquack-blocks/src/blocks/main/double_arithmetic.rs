//! `double_arithmetic`: double `+ - * /`; division by zero raises an error as in Snowflake.

use sqlquack_core::failures::constants::DIVISION_BY_ZERO;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::tagged_error;
use crate::blocks::constants::DOUBLE_ARITHMETIC;

/// Double `+ - * /`; division by zero raises an error as in Snowflake.
pub fn double_arithmetic(operator: &str, left: &str, right: &str, log: &ProvenanceLog) -> String {
    log.building_block(DOUBLE_ARITHMETIC);
    let left = format!("CAST({left} AS DOUBLE)");
    let right = format!("CAST({right} AS DOUBLE)");
    match operator {
        "/" => format!(
            "(CASE WHEN {right} = 0 THEN {} ELSE {left} / {right} END)",
            tagged_error(DIVISION_BY_ZERO, "Division by zero")
        ),
        _ => format!("({left} {operator} {right})"),
    }
}
