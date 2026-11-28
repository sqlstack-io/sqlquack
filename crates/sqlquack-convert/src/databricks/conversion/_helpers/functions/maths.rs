//! DEGREES and RADIANS read as DOUBLE, which compute as Java's Math.toDegrees and Math.toRadians; EXP, LN, LOG and the trigonometric functions round unlike DuckDB's libm.

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::functions::numbers::double;
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// The rewrite of the floating-point function `name`, `None` when no rewrite handles it.
pub(crate) fn math_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "DEGREES" => |values| unary(values, "degrees"),
        "RADIANS" => |values| unary(values, "radians"),
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
