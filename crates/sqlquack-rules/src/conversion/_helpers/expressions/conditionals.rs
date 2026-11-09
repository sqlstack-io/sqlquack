//! CASE, COALESCE, NVL, IFF, DECODE, ZEROIFNULL and the unification of their branches.

use serde_json::{Value, json};
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_semantics::types::main::utc_kept::utc_kept;
use sqlquack_snowflake_types::typing::main::text_reading::text_reading;
use sqlquack_snowflake_types::typing::main::unify::unify;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::casts::cast_to;
use crate::conversion::_helpers::expressions::operators::{boolean_operand, comparable};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::temporal::unified_cast;
use crate::conversion::_helpers::support::declared::declared_of;
use crate::conversion::_helpers::support::divergence::require_widening_range;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, list, node_construct, node_kind, payload_object, require_understood,
};
use crate::conversion::constants::{
    CASE, CASE_SUBJECT, COALESCE, DECODE_RULE, DIV0_RULE, IFF, NULL_BRANCHES, NULL_IF_ZERO_RULE,
    NVL2_RULE, SEMI_STRUCTURED_HOLDING, TEXT_BRANCHES, ZERO_IF_NULL_RULE,
};

/// The branches cast to their unified type, and that type; text beside other types refuses.
pub(crate) fn unified(
    branches: &[Emitted],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Vec<String>, SnowflakeType), Refusal> {
    unified_branches(branches, TextReading::Refused, (construct, context))
}

/// How a construct reads text branches beside other types.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextReading {
    /// Text beside other types refuses.
    Refused,
    /// Every text branch is read for every row, as COALESCE was recorded reading it.
    Eager,
    /// Only the chosen branch is read, as IFF, CASE and DECODE were recorded reading it.
    Lazy,
}

/// As `unified`, reading only the chosen text branch, as IFF, CASE and DECODE were recorded.
pub(crate) fn unified_reading_lazily(
    branches: &[Emitted],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Vec<String>, SnowflakeType), Refusal> {
    unified_branches(branches, TextReading::Lazy, (construct, context))
}

/// As `unified`, reading text beside other types as recorded, for a construct choosing a branch every row.
pub(crate) fn unified_reading_text(
    branches: &[Emitted],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Vec<String>, SnowflakeType), Refusal> {
    unified_branches(branches, TextReading::Eager, (construct, context))
}

fn unified_branches(
    branches: &[Emitted],
    reads_text: TextReading,
    (construct, context): (&Construct, &Context<'_>),
) -> Result<(Vec<String>, SnowflakeType), Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let reading = match reads_text != TextReading::Refused {
        true => text_reading(
            &branches
                .iter()
                .map(|branch| branch.data_type)
                .collect::<Vec<SnowflakeType>>(),
            context.log,
        ),
        false => None,
    };
    let read = |branch: &Emitted| {
        branch.data_type == SnowflakeType::Varchar && branch.literal.is_none() && reading.is_some()
    };
    let types: Vec<SnowflakeType> = branches
        .iter()
        .map(|branch| match (read(branch), reading) {
            (true, Some(reading)) => reading,
            _ => branch.data_type,
        })
        .collect();
    let target = unify(&types, context.log).map_err(refuse)?;
    require_widening_range(&types, construct, context)?;
    if target == SnowflakeType::Null {
        let _rule = context.log.rule(NULL_BRANCHES);
        let null = cast("NULL", SnowflakeType::Varchar, context.log).map_err(refuse)?;
        return Ok((vec![null; branches.len()], target));
    }
    let mut cast_branches: Vec<String> = Vec::with_capacity(branches.len());
    let mut readings: Vec<String> = Vec::new();
    for branch in branches {
        let branch = match (read(branch), reading) {
            (true, Some(reading)) => {
                let text = read_text(branch, (reading, target), construct, context)?;
                readings.push(text.sql.clone());
                &text.clone()
            }
            _ => branch,
        };
        cast_branches.push(match (target, branch.data_type) {
            (SnowflakeType::SemiStructured, SnowflakeType::SemiStructured) => branch.sql.clone(),
            (SnowflakeType::SemiStructured, _) => {
                format!("CAST(NULL AS {SEMI_STRUCTURED_HOLDING})")
            }
            _ => unified_cast(
                &branch.sql,
                (branch.data_type, target),
                (context, construct),
            )
            .map_err(refuse)?,
        });
    }
    match reads_text {
        TextReading::Eager => Ok((eagerly_read(cast_branches, &readings), target)),
        _ => Ok((cast_branches, target)),
    }
}

/// Branches that each first read every text branch, since Snowflake reads all of them for every row.
fn eagerly_read(branches: Vec<String>, readings: &[String]) -> Vec<String> {
    if readings.is_empty() {
        return branches;
    }
    let texts: Vec<String> = readings
        .iter()
        .map(|reading| format!("CAST({reading} AS VARCHAR)"))
        .collect();
    let guard = format!("length(concat_ws('', {})) >= 0", texts.join(", "));
    branches
        .into_iter()
        .map(|branch| format!("(CASE WHEN {guard} THEN {branch} END)"))
        .collect()
}

/// A text branch cast as Snowflake reads it beside other types, before widening to `target`.
pub(crate) fn read_text(
    branch: &Emitted,
    (reading, target): (SnowflakeType, SnowflakeType),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    if let (
        SnowflakeType::Number {
            scale: read_scale, ..
        },
        SnowflakeType::Number { scale, .. },
    ) = (reading, target)
        && scale > read_scale
    {
        return Err(refuse(format!(
            "text beside a {target}, whose reading at more than {read_scale} fraction digits no \
             recording shows"
        )));
    }
    let _rule = context.log.rule(TEXT_BRANCHES);
    let sql = cast_to(branch, &reading.to_string(), false, (context, construct)).map_err(refuse)?;
    Ok(Emitted::new(sql, reading))
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
    let (cast_branches, target) = unified_reading_text(&branches, construct, site.context)?;
    Ok(Emitted {
        utc_offset: utc_kept(&marks(&branches), target),
        declared: declared_of(branches.iter()),
        ..Emitted::new(format!("COALESCE({})", cast_branches.join(", ")), target)
    })
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
    let (cast_branches, target) = unified_reading_lazily(&branches, &construct, site.context)?;
    Ok(Emitted {
        utc_offset: utc_kept(&marks(&branches), target),
        declared: declared_of(branches.iter()),
        ..Emitted::new(
            format!(
                "(CASE WHEN {condition} THEN {} ELSE {} END)",
                cast_branches[0], cast_branches[1]
            ),
            target,
        )
    })
}

/// NVL2: the second argument where the first is not NULL, else the third; text beside other types refuses.
pub(crate) fn nvl2(
    (tested, when_set, otherwise): (&Value, &Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(NVL2_RULE);
    let tested = convert_expression(tested, site)?;
    if tested.data_type == SnowflakeType::SemiStructured {
        return Err(outside_conditions(
            construct.clone(),
            "NVL2 of a semi-structured value, whose JSON null no recording shows".to_owned(),
        ));
    }
    let branches = [
        convert_expression(when_set, site)?,
        convert_expression(otherwise, site)?,
    ];
    let (cast_branches, target) = unified(&branches, construct, site.context)?;
    Ok(Emitted::new(
        format!(
            "(CASE WHEN ({}) IS NOT NULL THEN {} ELSE {} END)",
            tested.sql, cast_branches[0], cast_branches[1]
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
    let whens = list(payload, "whens");
    let operand_node = payload.get("operand").filter(|operand| !operand.is_null());
    let (subject, conditions, mut branches) = match operand_node {
        Some(node) if !plain(node) && whens.iter().all(|when| when.get(0).is_some_and(plain)) => {
            let (subject, results) = bound_whens(node, whens, site)?;
            let positions = (1..=results.len()).map(|position| position.to_string());
            (format!(" {subject}"), positions.collect(), results)
        }
        Some(node) => {
            let operand = convert_expression(node, site)?;
            let (conditions, results) = case_whens(whens, Some(&operand), site)?;
            (String::new(), conditions, results)
        }
        None => {
            let (conditions, results) = case_whens(whens, None, site)?;
            (String::new(), conditions, results)
        }
    };
    let otherwise = match payload.get("else_").filter(|value| !value.is_null()) {
        Some(otherwise) => Some(convert_expression(otherwise, site)?),
        None => None,
    };
    branches.extend(otherwise.iter().cloned());
    let (cast_branches, target) = unified_reading_lazily(&branches, &construct, site.context)?;
    let mut sql = format!("(CASE{subject}");
    for (condition, result) in conditions.iter().zip(&cast_branches) {
        sql.push_str(&format!(" WHEN {condition} THEN {result}"));
    }
    if let (Some(_), Some(last)) = (&otherwise, cast_branches.last()) {
        sql.push_str(&format!(" ELSE {last}"));
    }
    sql.push_str(" END)");
    let mut values = marks(&branches);
    if otherwise.is_none() {
        values.push((SnowflakeType::Null, false));
    }
    Ok(Emitted {
        utc_offset: utc_kept(&values, target),
        declared: declared_of(branches.iter()),
        ..Emitted::new(sql, target)
    })
}

/// Each branch's type, and whether it is known to hold the UTC offset.
fn marks(branches: &[Emitted]) -> Vec<(SnowflakeType, bool)> {
    branches
        .iter()
        .map(|branch| (branch.data_type, branch.utc_offset))
        .collect()
}

/// Whether evaluating `node` is cheap to repeat and cannot fail: a column or a literal.
fn plain(node: &Value) -> bool {
    match node_kind(node) {
        Some(("column" | "literal" | "null" | "boolean", _)) => true,
        Some(("neg" | "paren", payload)) => payload.get("this").is_some_and(plain),
        _ => false,
    }
}

/// A simple CASE's operand bound once beside its WHEN values: the first equal WHEN's position.
fn bound_whens(
    operand: &Value,
    whens: &[Value],
    site: Site<'_>,
) -> Result<(String, Vec<Emitted>), Refusal> {
    let construct = node_construct("case");
    let operand = convert_expression(operand, site)?;
    let mut packed = vec![format!("o := {}", operand.sql)];
    let subject = Emitted {
        sql: format!("{CASE_SUBJECT}['o']"),
        ..operand
    };
    let mut tests: Vec<String> = Vec::new();
    let mut results: Vec<Emitted> = Vec::new();
    for (position, when) in whens.iter().enumerate() {
        let [value, result] = when.as_array().map_or(&[][..], Vec::as_slice) else {
            return Err(missing(
                construct,
                "a WHEN has an unexpected shape".to_owned(),
            ));
        };
        let value = convert_expression(value, site)?;
        packed.push(format!("w{position} := {}", value.sql));
        let value = Emitted {
            sql: format!("{CASE_SUBJECT}['w{position}']"),
            ..value
        };
        let (left, right) = comparable(subject.clone(), value, &construct, site.context)?;
        tests.push(format!(
            " WHEN {} = {} THEN {}",
            left.sql,
            right.sql,
            position + 1
        ));
        results.push(convert_expression(result, site)?);
    }
    if tests.is_empty() {
        return Err(missing(construct, "CASE without WHEN".to_owned()));
    }
    Ok((
        format!(
            "list_transform([struct_pack({})], lambda {CASE_SUBJECT}: CASE{} END)[1]",
            packed.join(", "),
            tests.concat()
        ),
        results,
    ))
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
    let (cast_branches, target) = unified_reading_lazily(&branches, construct, site.context)?;
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

/// DIV0 (DIV0NULL with `null_divisor`): zero for a zero divisor, NULL for a NULL dividend.
pub(crate) fn div0(
    arguments: &[Value],
    null_divisor: bool,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(DIV0_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [dividend, divisor] = arguments else {
        return Err(refuse(format!("{} with these arguments", construct.name)));
    };
    let quotient = convert_expression(&json!({"div": {"left": dividend, "right": divisor}}), site)?;
    let dividend = convert_expression(dividend, site)?;
    let divisor = convert_expression(divisor, site)?;
    let zero = cast("0", quotient.data_type, site.context.log).map_err(refuse)?;
    let null_test = match null_divisor {
        true => format!(" OR {} IS NULL", divisor.sql),
        false => String::new(),
    };
    if let Some(Literal::Number(digits)) = &divisor.literal
        && digits.trim_start_matches(['0', '.']).is_empty()
    {
        return Ok(Emitted::new(
            format!(
                "(CASE WHEN {} IS NULL THEN NULL ELSE {zero} END)",
                dividend.sql
            ),
            quotient.data_type,
        ));
    }
    Ok(Emitted::new(
        format!(
            "(CASE WHEN {} IS NULL THEN NULL WHEN {} = 0{null_test} THEN {zero} ELSE {} END)",
            dividend.sql, divisor.sql, quotient.sql
        ),
        quotient.data_type,
    ))
}

/// NULLIFZERO of a NUMBER or FLOAT: NULL for zero, otherwise the value, of its type.
pub(crate) fn null_if_zero(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(NULL_IF_ZERO_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse("NULLIFZERO with these arguments".to_owned()));
    };
    let operand = convert_expression(value, site)?;
    match operand.data_type {
        SnowflakeType::Number { .. } | SnowflakeType::Float => Ok(Emitted::new(
            format!("nullif({}, 0)", operand.sql),
            operand.data_type,
        )),
        other => Err(refuse(format!("NULLIFZERO of {other}"))),
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
