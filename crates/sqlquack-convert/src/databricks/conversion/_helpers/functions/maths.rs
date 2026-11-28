//! EXP, LN and LOG read as DOUBLE, as Snowflake's rules keep them, and the exact DEGREES and RADIANS; other floating-point functions round unlike DuckDB.

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::functions::numbers::double;
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// The rewrite of the floating-point function `name`, `None` when no rewrite handles it.
pub(crate) fn math_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "EXP" => |values| unary(values, "exp"),
        "DEGREES" => |values| unary(values, "degrees"),
        "RADIANS" => |values| unary(values, "radians"),
        "LN" => |values| logarithm(values, "ln"),
        "LOG" => base_logarithm,
        _ => return None,
    };
    Some(rewrite)
}

/// `function(x)` of one number read as DOUBLE.
fn unary(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [value] = values else { return None };
    Some(typed(
        format!("{function}({})", double(value)?),
        DatabricksType::Double,
    ))
}

/// LN of one number, NULL at or below zero as Databricks returns it, where DuckDB gives -inf or NaN.
fn logarithm(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [value] = values else { return None };
    let x = double(value)?;
    Some(typed(
        format!("CASE WHEN {x} <= 0 THEN NULL ELSE {function}({x}) END"),
        DatabricksType::Double,
    ))
}

/// `LOG(base, x)` as Databricks computes it, `ln(x) / ln(base)`, NULL when either is at or below zero.
fn base_logarithm(values: &[Emitted]) -> Option<Emitted> {
    let [base, value] = values else { return None };
    let (b, x) = (double(base)?, double(value)?);
    Some(typed(
        format!("CASE WHEN {b} <= 0 OR {x} <= 0 THEN NULL ELSE ln({x}) / ln({b}) END"),
        DatabricksType::Double,
    ))
}
