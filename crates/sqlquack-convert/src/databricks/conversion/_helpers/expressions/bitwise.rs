//! Bitwise operators, unary plus, `||`, and `IS [NOT] TRUE` or `FALSE`.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::{as_type, boolean, operands};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{field, flag, refused, understood};
use crate::databricks::conversion::_helpers::support::typing::wider_integer;
use crate::databricks::conversion::constants::{BITWISE_RULE, BOOLEAN_RULE, CONCAT_RULE};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Shift};

/// `&`, `|` or `^` on integers; `^` is XOR in Databricks, so it becomes DuckDB's `xor`.
pub(crate) fn bitwise(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands(kind, payload, site)?;
    let _rule = site.log.rule(BITWISE_RULE);
    let Some(wider) = wider_integer(left.data_type, right.data_type) else {
        return Err(refused(kind, format!("only {kind} of integers converts")));
    };
    let (left, right) = (as_type(&left, wider), as_type(&right, wider));
    let sql = match kind {
        "bitwise_and" => format!("({left} & {right})"),
        "bitwise_or" => format!("({left} | {right})"),
        _ => format!("xor({left}, {right})"),
    };
    Ok(typed(sql, wider))
}

/// `<<` or `>>` on an integer by an INT count, wrapping and masking the count as Java does.
pub(crate) fn shift(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (value, count) = operands(kind, payload, site)?;
    let _rule = site.log.rule(BITWISE_RULE);
    let direction = match kind {
        "bitwise_left_shift" => Shift::Left,
        _ => Shift::Right,
    };
    shifted(&value, &count, direction).ok_or_else(|| {
        refused(
            kind,
            format!(
                "no rule converts {} {kind} {}",
                value.data_type.name(),
                count.data_type.name()
            ),
        )
    })
}

/// `value` shifted by `count` as Java shifts an int or long: the count masked to the width, bits wrapping.
pub(crate) fn shifted(value: &Emitted, count: &Emitted, direction: Shift) -> Option<Emitted> {
    use DatabricksType as T;
    let count_fits = matches!(count.data_type, T::TinyInt | T::SmallInt | T::Int);
    let (result, wide, bits) = match value.data_type {
        T::TinyInt | T::SmallInt | T::Int => (T::Int, "BIGINT", 32_u32),
        T::BigInt => (T::BigInt, "HUGEINT", 64),
        _ => return None,
    };
    if !count_fits {
        return None;
    }
    let x = as_type(value, result);
    let s = format!("(CAST({} AS INTEGER) & {})", count.sql, bits - 1);
    let mask = format!("CAST('{}' AS {wide})", (1_u128 << bits) - 1);
    let unsigned = match direction {
        Shift::Right => return Some(typed(format!("({x} >> {s})"), result)),
        Shift::Left => format!("(((CAST({x} AS {wide}) & {mask}) << {s}) & {mask})"),
        Shift::RightUnsigned => format!("((CAST({x} AS {wide}) & {mask}) >> {s})"),
    };
    Some(typed(
        format!(
            "CAST({unsigned} - CASE WHEN {unsigned} >= CAST('{sign}' AS {wide}) THEN CAST('{span}' AS {wide}) \
             ELSE 0 END AS {})",
            result.duckdb_name(),
            sign = 1_u128 << (bits - 1),
            span = 1_u128 << bits
        ),
        result,
    ))
}

/// `~` on an integer.
pub(crate) fn bitwise_not(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let operand = single(payload, "bitwise_not", site)?;
    let _rule = site.log.rule(BITWISE_RULE);
    match operand.data_type.is_integral() {
        true => Ok(typed(format!("(~ {})", operand.sql), operand.data_type)),
        false => Err(refused(
            "bitwise_not",
            "only ~ of an integer converts".to_owned(),
        )),
    }
}

/// Unary `+`, which returns a number unchanged.
pub(crate) fn plus(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let operand = single(payload, "pos", site)?;
    match operand.data_type.is_numeric() {
        true => Ok(typed(operand.sql, operand.data_type)),
        false => Err(refused("pos", "only + of a number converts".to_owned())),
    }
}

/// `a || b` on strings, NULL when either is NULL in both engines.
pub(crate) fn concat_operator(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let (left, right) = operands("concat", payload, site)?;
    let _rule = site.log.rule(CONCAT_RULE);
    let text = |data_type| matches!(data_type, DatabricksType::String | DatabricksType::Null);
    match text(left.data_type) && text(right.data_type) {
        true => Ok(typed(
            format!("({} || {})", left.sql, right.sql),
            DatabricksType::String,
        )),
        false => Err(refused("concat", "only || of strings converts".to_owned())),
    }
}

/// `x IS [NOT] TRUE` or `x IS [NOT] FALSE`, never NULL.
pub(crate) fn truth_test(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let test = understood(payload, kind, &["this", "not"])?;
    let operand = boolean(&expression(field(test, "this"), site)?, kind)?;
    let _rule = site.log.rule(BOOLEAN_RULE);
    let not = match flag(test, "not") {
        true => " NOT",
        false => "",
    };
    let value = match kind {
        "is_true" => "TRUE",
        _ => "FALSE",
    };
    Ok(typed(
        format!("({operand} IS{not} {value})"),
        DatabricksType::Boolean,
    ))
}

fn single(payload: &Value, kind: &str, site: Site<'_>) -> Result<Emitted, Refusal> {
    let node = understood(payload, kind, &["this"])?;
    expression(field(node, "this"), site)
}
