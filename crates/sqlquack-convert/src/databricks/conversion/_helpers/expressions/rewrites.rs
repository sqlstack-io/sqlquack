//! Functions DuckDB names or bounds differently, rewritten to give Databricks's results exactly.

use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::arrays::array_rewrite;
use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::{as_type, common_type};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::refused_function;
use crate::databricks::conversion::_helpers::support::typing::wider_integer;
use crate::databricks::conversion::constants::{REWRITE_RULE, SHA2_DEFAULT_BITS, SHA2_SHA256_BITS};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// A rewrite: the DuckDB expression for the converted arguments, `None` when they are outside it.
pub(crate) type Rewrite = fn(&[Emitted]) -> Option<Emitted>;

/// The rewrite of the function `name`, `None` when no rewrite handles it.
pub(crate) fn rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "NVL" | "IFNULL" => first_not_null,
        "NVL2" => nvl2,
        "LEFT" => left,
        "REPEAT" => repeat,
        "HEX" => hex,
        "SHA2" => sha2,
        "TRY_ADD" | "TRY_SUBTRACT" | "TRY_MULTIPLY" => match name {
            "TRY_ADD" => try_add,
            "TRY_SUBTRACT" => try_subtract,
            _ => try_multiply,
        },
        "TRY_DIVIDE" => try_divide,
        "TRY_MOD" => try_mod,
        _ => return array_rewrite(name),
    };
    Some(rewrite)
}

/// `rewrite` applied to the converted arguments of `name`, or the refusal of arguments outside it.
pub(crate) fn applied(
    name: &str,
    rewrite: Rewrite,
    values: &[Emitted],
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.log.rule(REWRITE_RULE);
    rewrite(values).ok_or_else(|| {
        let names: Vec<String> = values.iter().map(|value| value.data_type.name()).collect();
        refused_function(
            name,
            format!(
                "no rule converts {name} of ({}): the arguments are outside its rewrite",
                names.join(", ")
            ),
        )
    })
}

/// `NVL(a, b)` and `IFNULL(a, b)`: `coalesce` over one type.
fn first_not_null(values: &[Emitted]) -> Option<Emitted> {
    let [_, _] = values else { return None };
    let common = known(common_type(values)?)?;
    let sqls: Vec<String> = values.iter().map(|value| as_type(value, common)).collect();
    Some(typed(format!("coalesce({})", sqls.join(", ")), common))
}

/// `NVL2(a, b, c)`: `b` when `a` is not NULL, else `c`.
fn nvl2(values: &[Emitted]) -> Option<Emitted> {
    let [test, present, absent] = values else {
        return None;
    };
    let common = known(common_type(&values[1..])?)?;
    Some(typed(
        format!(
            "CASE WHEN {} IS NOT NULL THEN {} ELSE {} END",
            test.sql,
            as_type(present, common),
            as_type(absent, common)
        ),
        common,
    ))
}

/// `LEFT(s, n)`: '' when `n` is zero or negative, where DuckDB would count from the end.
fn left(values: &[Emitted]) -> Option<Emitted> {
    let [text, length] = values else { return None };
    match (text.data_type, small_integer(length.data_type)) {
        (DatabricksType::String, true) => Some(typed(
            format!(
                "CASE WHEN {text} IS NULL THEN NULL WHEN {length} <= 0 THEN '' ELSE left({text}, {length}) END",
                text = text.sql,
                length = length.sql
            ),
            DatabricksType::String,
        )),
        _ => None,
    }
}

/// `REPEAT(s, n)`, '' for a zero or negative count in both engines.
fn repeat(values: &[Emitted]) -> Option<Emitted> {
    let [text, count] = values else { return None };
    match (text.data_type, small_integer(count.data_type)) {
        (DatabricksType::String, true) => Some(typed(
            format!("repeat({}, {})", text.sql, count.sql),
            DatabricksType::String,
        )),
        _ => None,
    }
}

/// `HEX` of a string's UTF-8 bytes, or of an integer as 64-bit two's complement, upper case.
fn hex(values: &[Emitted]) -> Option<Emitted> {
    let [value] = values else { return None };
    let sql = match value.data_type {
        DatabricksType::String => value.sql.clone(),
        data_type if data_type.is_integral() => as_type(value, DatabricksType::BigInt),
        _ => return None,
    };
    Some(typed(format!("hex({sql})"), DatabricksType::String))
}

/// `SHA2(s, 256)` and `SHA2(s, 0)`: DuckDB's lower-case `sha256`.
fn sha2(values: &[Emitted]) -> Option<Emitted> {
    let [text, bits] = values else { return None };
    let bits = match &bits.literal {
        Some(Literal::Number(bits)) => bits.as_str(),
        _ => return None,
    };
    match (
        text.data_type,
        [SHA2_SHA256_BITS, SHA2_DEFAULT_BITS].contains(&bits),
    ) {
        (DatabricksType::String, true) => Some(typed(
            format!("sha256({})", text.sql),
            DatabricksType::String,
        )),
        _ => None,
    }
}

fn try_add(values: &[Emitted]) -> Option<Emitted> {
    try_integer(values, "+")
}

fn try_subtract(values: &[Emitted]) -> Option<Emitted> {
    try_integer(values, "-")
}

fn try_multiply(values: &[Emitted]) -> Option<Emitted> {
    try_integer(values, "*")
}

/// `TRY_ADD`, `TRY_SUBTRACT` or `TRY_MULTIPLY` of integers: NULL on overflow through DuckDB's `try`.
fn try_integer(values: &[Emitted], symbol: &str) -> Option<Emitted> {
    let [left, right] = values else { return None };
    let wider = wider_integer(left.data_type, right.data_type)?;
    Some(typed(
        format!(
            "try({} {symbol} {})",
            as_type(left, wider),
            as_type(right, wider)
        ),
        wider,
    ))
}

/// `TRY_DIVIDE` of integers: a DOUBLE quotient, NULL for a zero divisor where DuckDB gives NaN or infinity.
fn try_divide(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    wider_integer(left.data_type, right.data_type)?;
    let divisor = as_type(right, DatabricksType::Double);
    Some(typed(
        format!(
            "CASE WHEN {divisor} = 0 THEN NULL ELSE {} / {divisor} END",
            as_type(left, DatabricksType::Double)
        ),
        DatabricksType::Double,
    ))
}

/// `TRY_MOD` of integers: NULL for a zero divisor, and 0 by -1, where DuckDB overflows on the minimum.
fn try_mod(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    let wider = wider_integer(left.data_type, right.data_type)?;
    let (dividend, divisor) = (as_type(left, wider), as_type(right, wider));
    Some(typed(
        format!(
            "CASE WHEN {divisor} = -1 THEN {dividend} * 0 ELSE try({dividend} % {divisor}) END"
        ),
        wider,
    ))
}

/// A count argument Databricks takes as INT without a cast.
fn small_integer(data_type: DatabricksType) -> bool {
    matches!(
        data_type,
        DatabricksType::TinyInt | DatabricksType::SmallInt | DatabricksType::Int
    )
}

/// A type other than the bare NULL's.
fn known(data_type: DatabricksType) -> Option<DatabricksType> {
    match data_type {
        DatabricksType::Null => None,
        data_type => Some(data_type),
    }
}
