//! VARIANT values held as DuckDB `JSON`: parsing, paths, inspection and casts.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::varchar_length_check::varchar_length_check;
use sqlquack_duckdb::semi_structured::constants::VARIANT_TYPED_VALUES;
use sqlquack_duckdb::semi_structured::constants::{JSON_FIDELITY, JSON_NULL};
use sqlquack_duckdb::semi_structured::main::json_get_ignore_case::json_get_ignore_case;
use sqlquack_duckdb::semi_structured::main::json_guarded::json_guarded;
use sqlquack_duckdb::semi_structured::main::json_null_value::json_null_value;
use sqlquack_duckdb::semi_structured::main::json_path::json_path;
use sqlquack_duckdb::semi_structured::main::json_scalar::json_scalar;
use sqlquack_duckdb::semi_structured::main::json_serialized::json_serialized;
use sqlquack_duckdb::semi_structured::main::json_text::json_text;
use sqlquack_duckdb::semi_structured::models::PathStep;
use sqlquack_duckdb::variants::main::json_type_name::json_type_name;
use sqlquack_snowflake::typing::models::{CastTarget, SnowflakeType};

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::support::divergence::{allows, diverge};
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    PARSE_JSON_RULE, TIMESTAMP_RANGE, VARIANT_CAST, VARIANT_INSPECT, VARIANT_NAME, VARIANT_PATH,
    VARIANT_TYPE_NAME,
};

/// PARSE_JSON or TRY_PARSE_JSON of text: raises wherever the two parsers may disagree.
pub(crate) fn parse_json(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(PARSE_JSON_RULE);
    let [text] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "parsing JSON takes one argument here".to_owned(),
        ));
    };
    let text = text_operand(text, construct, site)?;
    diverge(site.context, JSON_FIDELITY, construct)?;
    Ok(semi_structured(json_guarded(&text, site.context.log)))
}

/// A VARIANT column of a base table, guarded as PARSE_JSON's result is.
pub(crate) fn guarded_column(sql: &str, context: &Context<'_>) -> Result<String, Refusal> {
    diverge(context, JSON_FIDELITY, &node_construct("column"))?;
    Ok(json_guarded(sql, context.log))
}

/// `v:a.b[0]`: a literal path into a VARIANT.
pub(crate) fn variant_path(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("j_s_o_n_extract");
    let _rule = site.context.log.rule(VARIANT_PATH);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["this", "expression", "variant_extract"],
        &construct,
    )?;
    let base = variant_operand(field(payload, "this", &construct)?, &construct, site)?;
    let path = field(payload, "expression", &construct)?;
    let text = path
        .get("literal")
        .and_then(|literal| literal.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            outside_conditions(construct.clone(), "a path other than a literal".to_owned())
        })?;
    let steps = path_steps(text).ok_or_else(|| {
        outside_conditions(
            construct.clone(),
            format!("the path `{text}` is not plain keys and indexes"),
        )
    })?;
    located(&base, &steps, &construct, site.context)
}

/// `v[0]` or `v['key']` on a VARIANT.
pub(crate) fn variant_subscript(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("subscript");
    let _rule = site.context.log.rule(VARIANT_PATH);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "index"], &construct)?;
    let base = variant_operand(field(payload, "this", &construct)?, &construct, site)?;
    let step = literal_step(field(payload, "index", &construct)?, &construct, site)?;
    located(&base, &[step], &construct, site.context)
}

/// GET, GET_PATH and GET_IGNORE_CASE with literal keys, indexes and paths.
pub(crate) fn variant_get(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(VARIANT_PATH);
    let [value, key] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} takes two arguments"),
        ));
    };
    let base = variant_operand(value, construct, site)?;
    match name {
        "GET" => {
            let step = literal_step(key, construct, site)?;
            located(&base, &[step], construct, site.context)
        }
        "GET_PATH" => {
            let text = text_literal_value(key, construct, site)?;
            let steps = path_steps(&text).ok_or_else(|| {
                outside_conditions(construct.clone(), format!("the path `{text}` is not plain"))
            })?;
            located(&base, &steps, construct, site.context)
        }
        _ => {
            let text = text_literal_value(key, construct, site)?;
            diverge(site.context, JSON_FIDELITY, construct)?;
            json_get_ignore_case(&base, &text, site.context.log)
                .map(semi_structured)
                .map_err(|reason| outside_conditions(construct.clone(), reason))
        }
    }
}

/// TYPEOF, IS_NULL_VALUE and TO_JSON of a VARIANT.
pub(crate) fn variant_inspect(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let [value] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} takes one argument"),
        ));
    };
    let base = variant_operand(value, construct, site)?;
    match name {
        "TYPEOF" => {
            let _rule = log.rule(VARIANT_TYPE_NAME);
            Ok(Emitted::new(
                json_type_name(
                    &base,
                    allows(site.context, VARIANT_TYPED_VALUES, construct),
                    log,
                ),
                SnowflakeType::Varchar,
            ))
        }
        "IS_NULL_VALUE" => {
            let _rule = log.rule(VARIANT_INSPECT);
            Ok(Emitted::new(
                json_null_value(&base, log),
                SnowflakeType::Boolean,
            ))
        }
        _ => {
            let _rule = log.rule(VARIANT_INSPECT);
            diverge(site.context, JSON_FIDELITY, construct)?;
            Ok(Emitted::new(
                json_serialized(&base, log),
                SnowflakeType::Varchar,
            ))
        }
    }
}

/// A VARIANT cast to VARIANT unchanged, or to a scalar type; objects and arrays raise.
pub(crate) fn variant_cast(
    operand: &Emitted,
    (type_name, target): (&str, CastTarget),
    trying: bool,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<String, String> {
    let _rule = context.log.rule(VARIANT_CAST);
    if trying {
        return Err("TRY_CAST of a VARIANT has no proven conversion".to_owned());
    }
    if type_name == VARIANT_NAME {
        return Ok(operand.sql.clone());
    }
    diverge(context, JSON_FIDELITY, construct).map_err(|refusal| refusal.reason)?;
    if matches!(target.data_type, SnowflakeType::TimestampNtz { .. }) {
        diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason)?;
    }
    match (target.data_type, target.max_length) {
        (SnowflakeType::Varchar, Some(max_length)) => Ok(varchar_length_check(
            &json_text(&operand.sql, context.log),
            max_length,
            context.log,
        )),
        (SnowflakeType::Varchar, None) => Ok(json_text(&operand.sql, context.log)),
        (to, _) => json_scalar(&operand.sql, to, context.log),
    }
}

/// SQL NULL as a VARIANT.
pub(crate) fn variant_null(log: &ProvenanceLog) -> String {
    log.building_block(JSON_NULL);
    "CAST(NULL AS JSON)".to_owned()
}

fn semi_structured(sql: String) -> Emitted {
    Emitted::new(sql, SnowflakeType::SemiStructured)
}

fn variant_operand(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::SemiStructured => Ok(emitted.sql),
        other => Err(outside_conditions(
            construct.clone(),
            format!("a path into a {other} value, not a VARIANT"),
        )),
    }
}

fn located(
    base: &str,
    steps: &[PathStep],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    json_path(base, steps, context.log)
        .map(semi_structured)
        .map_err(|reason| outside_conditions(construct.clone(), reason))
}

fn literal_step(node: &Value, construct: &Construct, site: Site<'_>) -> Result<PathStep, Refusal> {
    match convert_expression(node, site)?.literal {
        Some(Literal::Text(key)) => Ok(PathStep::Key(key)),
        Some(Literal::Number(digits)) => digits.parse().map(PathStep::Index).map_err(|_| {
            outside_conditions(
                construct.clone(),
                format!("the index {digits} is not a whole number"),
            )
        }),
        _ => Err(outside_conditions(
            construct.clone(),
            "a key or index other than a literal".to_owned(),
        )),
    }
}

fn text_literal_value(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    match convert_expression(node, site)?.literal {
        Some(Literal::Text(text)) => Ok(text),
        _ => Err(outside_conditions(
            construct.clone(),
            "an argument other than a text literal".to_owned(),
        )),
    }
}

/// The steps of a Snowflake path: dotted keys, `[n]` indexes and `["key"]` keys.
fn path_steps(text: &str) -> Option<Vec<PathStep>> {
    let mut steps: Vec<PathStep> = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(bracketed) = rest.strip_prefix("[\"") {
            let end = bracketed.find("\"]")?;
            steps.push(PathStep::Key(bracketed[..end].to_owned()));
            rest = &bracketed[end + 2..];
        } else if let Some(bracketed) = rest.strip_prefix('[') {
            let end = bracketed.find(']')?;
            let digits = &bracketed[..end];
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let index = match digits.parse() {
                Ok(index) => index,
                Err(_) => return None,
            };
            steps.push(PathStep::Index(index));
            rest = &bracketed[end + 1..];
        } else {
            let key_text = match rest.strip_prefix('.') {
                Some(after) if !steps.is_empty() => after,
                Some(_) => return None,
                None if steps.is_empty() => rest,
                None => return None,
            };
            let end = key_text.find(['.', '[']).unwrap_or(key_text.len());
            let key = &key_text[..end];
            if key.is_empty() || key.contains(['"', ']', '*']) {
                return None;
            }
            steps.push(PathStep::Key(key.to_owned()));
            rest = &key_text[end..];
        }
    }
    (!steps.is_empty()).then_some(steps)
}
