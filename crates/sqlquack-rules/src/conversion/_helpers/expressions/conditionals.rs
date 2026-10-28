//! CASE, COALESCE, NVL, IFF, DECODE, ZEROIFNULL and the unification of their branches.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_snowflake::typing::main::unify::unify;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::operators::{boolean_operand, comparable};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::require_widening_range;
use crate::conversion::_helpers::support::scope::{Context, Emitted};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, list, missing, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    CASE, COALESCE, DECODE_RULE, IFF, SEMI_STRUCTURED_HOLDING, ZERO_IF_NULL_RULE,
};

/// The branches cast to their unified type, and that type.
pub(crate) fn unified(
    branches: &[Emitted],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Vec<String>, SnowflakeType), Refusal> {
    let types: Vec<SnowflakeType> = branches.iter().map(|branch| branch.data_type).collect();
    let target = unify(&types, context.log)
        .map_err(|reason| outside_conditions(construct.clone(), reason))?;
    require_widening_range(&types, construct, context)?;
    if target == SnowflakeType::Null {
        return Err(outside_conditions(
            construct.clone(),
            "every branch is an untyped NULL".to_owned(),
        ));
    }
    let mut cast_branches: Vec<String> = Vec::with_capacity(branches.len());
    for branch in branches {
        cast_branches.push(match (target, branch.data_type) {
            (SnowflakeType::SemiStructured, SnowflakeType::SemiStructured) => branch.sql.clone(),
            (SnowflakeType::SemiStructured, _) => {
                format!("CAST(NULL AS {SEMI_STRUCTURED_HOLDING})")
            }
            _ => cast(&branch.sql, target, context.log)
                .map_err(|reason| outside_conditions(construct.clone(), reason))?,
        });
    }
    Ok((cast_branches, target))
}

fn coalesce_arguments(
    arguments: &[&Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(COALESCE);
    let mut branches: Vec<Emitted> = Vec::with_capacity(arguments.len());
    for argument in arguments {
        branches.push(convert_expression(argument, site)?);
    }
    if branches.is_empty() {
        return Err(missing(
            construct.clone(),
            "COALESCE without arguments".to_owned(),
        ));
    }
    let (cast_branches, target) = unified(&branches, construct, site.context)?;
    Ok(Emitted::new(
        format!("COALESCE({})", cast_branches.join(", ")),
        target,
    ))
}

/// COALESCE and IFNULL.
pub(crate) fn coalesce(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("coalesce");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["expressions"], &construct)?;
    let arguments: Vec<&Value> = list(payload, "expressions").iter().collect();
    coalesce_arguments(&arguments, &construct, site)
}

/// NVL.
pub(crate) fn nvl(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("nvl");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "expression"], &construct)?;
    let arguments = [
        field(payload, "this", &construct)?,
        field(payload, "expression", &construct)?,
    ];
    coalesce_arguments(&arguments, &construct, site)
}

/// IFF.
pub(crate) fn iff(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("if_func");
    let _rule = site.context.log.rule(IFF);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["condition", "true_value", "false_value"],
        &construct,
    )?;
    let condition = boolean_operand(field(payload, "condition", &construct)?, site)?;
    let branches = [
        convert_expression(field(payload, "true_value", &construct)?, site)?,
        convert_expression(field(payload, "false_value", &construct)?, site)?,
    ];
    let (cast_branches, target) = unified(&branches, &construct, site.context)?;
    Ok(Emitted::new(
        format!(
            "(CASE WHEN {condition} THEN {} ELSE {} END)",
            cast_branches[0], cast_branches[1]
        ),
        target,
    ))
}

/// Searched and simple CASE.
pub(crate) fn case_expression(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("case");
    let _rule = site.context.log.rule(CASE);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["operand", "whens", "else_"], &construct)?;
    let operand = match payload.get("operand").filter(|operand| !operand.is_null()) {
        Some(operand) => Some(convert_expression(operand, site)?),
        None => None,
    };
    let (conditions, mut branches) = case_whens(list(payload, "whens"), operand.as_ref(), site)?;
    let otherwise = match payload.get("else_").filter(|value| !value.is_null()) {
        Some(otherwise) => Some(convert_expression(otherwise, site)?),
        None => None,
    };
    branches.extend(otherwise.iter().cloned());
    let (cast_branches, target) = unified(&branches, &construct, site.context)?;
    let mut sql = String::from("(CASE");
    for (condition, result) in conditions.iter().zip(&cast_branches) {
        sql.push_str(&format!(" WHEN {condition} THEN {result}"));
    }
    if let (Some(_), Some(last)) = (&otherwise, cast_branches.last()) {
        sql.push_str(&format!(" ELSE {last}"));
    }
    sql.push_str(" END)");
    Ok(Emitted::new(sql, target))
}

fn case_whens(
    whens: &[Value],
    operand: Option<&Emitted>,
    site: Site<'_>,
) -> Result<(Vec<String>, Vec<Emitted>), Refusal> {
    let construct = node_construct("case");
    let mut conditions: Vec<String> = Vec::new();
    let mut results: Vec<Emitted> = Vec::new();
    for when in whens {
        let [condition, result] = when.as_array().map_or(&[][..], Vec::as_slice) else {
            return Err(missing(
                construct,
                "a WHEN has an unexpected shape".to_owned(),
            ));
        };
        let condition = match operand {
            Some(operand) => {
                let value = convert_expression(condition, site)?;
                let (left, right) = comparable(operand.clone(), value, &construct, site.context)?;
                format!("{} = {}", left.sql, right.sql)
            }
            None => boolean_operand(condition, site)?,
        };
        conditions.push(condition);
        results.push(convert_expression(result, site)?);
    }
    if conditions.is_empty() {
        return Err(missing(construct, "CASE without WHEN".to_owned()));
    }
    Ok((conditions, results))
}

/// DECODE: the first result whose search equals the expression, NULL matching NULL.
pub(crate) fn decode(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DECODE_RULE);
    let [subject, _, _, ..] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "DECODE without a search and result".to_owned(),
        ));
    };
    let rest = &arguments[1..];
    let subject = convert_expression(subject, site)?;
    let mut tests: Vec<String> = Vec::new();
    let mut branches: Vec<Emitted> = Vec::new();
    for pair in rest.chunks(2) {
        match pair {
            [search, result] => {
                let search = convert_expression(search, site)?;
                if mixes_float_and_number(subject.data_type, search.data_type) {
                    return Err(outside_conditions(
                        construct.clone(),
                        "DECODE comparing FLOAT with NUMBER, which no recording proves near a \
                         double's precision"
                            .to_owned(),
                    ));
                }
                let (left, right) = comparable(subject.clone(), search, construct, site.context)?;
                tests.push(format!("{} IS NOT DISTINCT FROM {}", left.sql, right.sql));
                branches.push(convert_expression(result, site)?);
            }
            [default] => branches.push(convert_expression(default, site)?),
            _ => {}
        }
    }
    let (cast_branches, target) = unified(&branches, construct, site.context)?;
    let mut sql = String::from("(CASE");
    for (test, branch) in tests.iter().zip(&cast_branches) {
        sql.push_str(&format!(" WHEN {test} THEN {branch}"));
    }
    if let Some(default) = cast_branches.get(tests.len()) {
        sql.push_str(&format!(" ELSE {default}"));
    }
    sql.push_str(" END)");
    Ok(Emitted::new(sql, target))
}

/// ZEROIFNULL: the NUMBER or FLOAT value, or zero where it is NULL, of the value's type.
pub(crate) fn zero_if_null(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(ZERO_IF_NULL_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse("ZEROIFNULL with these arguments".to_owned()));
    };
    let operand = convert_expression(value, site)?;
    match operand.data_type {
        SnowflakeType::Number { .. } | SnowflakeType::Float => Ok(Emitted::new(
            cast(
                &format!("coalesce({}, 0)", operand.sql),
                operand.data_type,
                site.context.log,
            )
            .map_err(refuse)?,
            operand.data_type,
        )),
        other => Err(refuse(format!("ZEROIFNULL of {other}"))),
    }
}

/// Whether one of two types is FLOAT and the other NUMBER.
fn mixes_float_and_number(left: SnowflakeType, right: SnowflakeType) -> bool {
    matches!(
        (left, right),
        (SnowflakeType::Float, SnowflakeType::Number { .. })
            | (SnowflakeType::Number { .. }, SnowflakeType::Float)
    )
}
