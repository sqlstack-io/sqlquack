//! FILTER and TRANSFORM of an array with a one-parameter lambda; over SPLIT, a string parameter.

use serde_json::Value;
use sqlquack_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_blocks::semi_structured::main::json_filter::json_filter;
use sqlquack_blocks::semi_structured::main::json_reduce::json_reduce;
use sqlquack_blocks::semi_structured::main::json_transform::json_transform;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::objects::{
    as_json, semi_structured, variant,
};
use crate::snowflake::conversion::_helpers::expressions::scalar::{call_name, convert_expression};
use crate::snowflake::conversion::_helpers::support::divergence::diverge;
use crate::snowflake::conversion::_helpers::support::scope::{Emitted, Scope};
use crate::snowflake::conversion::_helpers::support::site::{Placement, Site};
use crate::snowflake::conversion::_helpers::support::tree::{identifier_name, list, node_kind};
use crate::snowflake::conversion::constants::{
    CALL_KIND, FILTER_FUNCTION, LAMBDA_KIND, LAMBDA_PARAMETER_PREFIX, LAMBDA_RULE, REDUCE_RULE,
    STRING_ARRAY_FUNCTION, TEXT_PARAMETER_TYPES,
};

/// FILTER(array, x -> predicate) or TRANSFORM(array, x -> value), the parameter untyped.
pub(crate) fn lambda_call(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(LAMBDA_RULE);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let [array, lambda] = arguments else {
        return Err(refuse("a lambda call with these arguments"));
    };
    let Some((LAMBDA_KIND, lambda)) = node_kind(lambda) else {
        return Err(refuse("a second argument other than a lambda"));
    };
    let declared = list(
        lambda
            .as_object()
            .ok_or_else(|| refuse("an unreadable lambda"))?,
        "parameter_types",
    );
    let text_typed = declared.iter().all(|declared| {
        declared.is_null()
            || (declared.get("length").is_none_or(Value::is_null)
                && declared
                    .get("data_type")
                    .and_then(Value::as_str)
                    .is_some_and(|name| TEXT_PARAMETER_TYPES.contains(&name)))
    });
    let typed = declared.iter().any(|declared| !declared.is_null());
    let parameters = lambda
        .get("parameters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let [parameter] = parameters else {
        return Err(refuse("a lambda with other than one parameter"));
    };
    if !text_typed || lambda.get("colon").and_then(Value::as_bool) == Some(true) {
        return Err(refuse("a lambda with a parameter typed other than VARCHAR"));
    }
    let parameter = identifier_name(parameter).ok_or_else(|| refuse("an unnamed parameter"))?;
    let body = lambda
        .get("body")
        .ok_or_else(|| refuse("a lambda without a body"))?;
    let strings = string_array(array);
    if typed && !strings {
        return Err(refuse(
            "a VARCHAR-typed parameter over an array not known to hold strings",
        ));
    }
    let reads = site.context.typed.reads();
    let array = variant(array, construct, site)?;
    let (untainted, sourced) = site.context.typed.since(reads);
    diverge(site.context, JSON_FIDELITY, construct)?;
    let bound = format!(
        "{LAMBDA_PARAMETER_PREFIX}{}",
        parameter
            .chars()
            .map(|character| match character.is_ascii_alphanumeric() {
                true => character.to_ascii_lowercase(),
                false => '_',
            })
            .collect::<String>()
    );
    let mut scope = Scope::new(Vec::new(), Some(site.scope));
    scope.aliases_first = true;
    let element = match strings {
        true => Emitted::new(
            format!("json_extract_string({bound}, '$')"),
            SnowflakeType::Varchar,
        ),
        false => Emitted::new(bound.clone(), SnowflakeType::SemiStructured),
    };
    let element = Emitted {
        untainted,
        sourced,
        ..element
    };
    scope.aliases.push((parameter, element));
    let inner = site.scoped(&scope).placed(Placement::ROW);
    let body = convert_expression(body, inner)?;
    let log = site.context.log;
    match name {
        "FILTER" => match body.data_type {
            SnowflakeType::Boolean | SnowflakeType::Null => Ok(semi_structured(json_filter(
                &array,
                (&bound, &body.sql),
                log,
            ))),
            other => Err(refuse(&format!("a FILTER predicate of type {other}"))),
        },
        _ => {
            let value = as_json(body, construct, site.context)?;
            Ok(semi_structured(json_transform(
                &array,
                (&bound, &value),
                log,
            )))
        }
    }
}

/// Whether every element of the array `node` is a string: SPLIT's, or FILTER's of such an array.
fn string_array(node: &Value) -> bool {
    match node_kind(node) {
        Some((CALL_KIND, payload)) => match call_name(payload).as_str() {
            STRING_ARRAY_FUNCTION => true,
            FILTER_FUNCTION => payload
                .get("args")
                .and_then(Value::as_array)
                .and_then(|arguments| arguments.first())
                .is_some_and(string_array),
            _ => false,
        },
        _ => false,
    }
}

/// REDUCE(array, initial, (acc, x) -> body) with a FLOAT accumulator, folded left in order.
pub(crate) fn reduce_call(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(REDUCE_RULE);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let [array, initial, lambda] = arguments else {
        return Err(refuse("REDUCE with these arguments"));
    };
    let Some((LAMBDA_KIND, lambda)) = node_kind(lambda) else {
        return Err(refuse("a third argument other than a lambda"));
    };
    let untyped = list(
        lambda
            .as_object()
            .ok_or_else(|| refuse("an unreadable lambda"))?,
        "parameter_types",
    )
    .iter()
    .all(Value::is_null);
    let parameters = lambda
        .get("parameters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let [accumulator, element] = parameters else {
        return Err(refuse("a REDUCE lambda with other than two parameters"));
    };
    if !untyped || lambda.get("colon").and_then(Value::as_bool) == Some(true) {
        return Err(refuse("a REDUCE lambda with typed parameters"));
    }
    let initial = convert_expression(initial, site)?;
    if initial.data_type != SnowflakeType::Float {
        return Err(refuse(&format!(
            "REDUCE from a {} initial value, whose accumulator type no recording shows",
            initial.data_type
        )));
    }
    let name = |node: &Value| identifier_name(node).ok_or_else(|| refuse("an unnamed parameter"));
    let (accumulator, element) = (name(accumulator)?, name(element)?);
    let bound = |parameter: &str| {
        format!(
            "{LAMBDA_PARAMETER_PREFIX}{}",
            parameter
                .chars()
                .map(|character| match character.is_ascii_alphanumeric() {
                    true => character.to_ascii_lowercase(),
                    false => '_',
                })
                .collect::<String>()
        )
    };
    let (accumulator_sql, element_sql) = (bound(&accumulator), bound(&element));
    if accumulator_sql == element_sql {
        return Err(refuse("REDUCE parameters that bind alike"));
    }
    let reads = site.context.typed.reads();
    let array = variant(array, construct, site)?;
    let (untainted, sourced) = site.context.typed.since(reads);
    diverge(site.context, JSON_FIDELITY, construct)?;
    let mut scope = Scope::new(Vec::new(), Some(site.scope));
    scope.aliases_first = true;
    scope.aliases.push((
        accumulator,
        Emitted::new(accumulator_sql.clone(), SnowflakeType::Float),
    ));
    scope.aliases.push((
        element,
        Emitted {
            untainted,
            sourced,
            ..Emitted::new(element_sql.clone(), SnowflakeType::SemiStructured)
        },
    ));
    let body = lambda
        .get("body")
        .ok_or_else(|| refuse("a lambda without a body"))?;
    let body = convert_expression(body, site.scoped(&scope).placed(Placement::ROW))?;
    if !matches!(body.data_type, SnowflakeType::Float | SnowflakeType::Null) {
        return Err(refuse(&format!(
            "a REDUCE body of type {}, not the FLOAT accumulator",
            body.data_type
        )));
    }
    Ok(Emitted::new(
        json_reduce(
            &array,
            (&accumulator_sql, &element_sql, &body.sql),
            &initial.sql,
            site.context.log,
        ),
        SnowflakeType::Float,
    ))
}
