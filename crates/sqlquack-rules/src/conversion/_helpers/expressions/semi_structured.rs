//! VARIANT values held as DuckDB `JSON`: parsing, paths, inspection and casts.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::varchar_length_check::varchar_length_check;
use sqlquack_duckdb::floats::constants::FLOAT_TEXT;
use sqlquack_duckdb::held::main::variant_double::variant_double;
use sqlquack_duckdb::held::main::variant_json::variant_json;
use sqlquack_duckdb::held::main::variant_text::variant_text;
use sqlquack_duckdb::held::main::variant_type_name::variant_type_name;
use sqlquack_duckdb::held::main::variant_written::variant_written;
use sqlquack_duckdb::semi_structured::constants::VARIANT_TYPED_VALUES;
use sqlquack_duckdb::semi_structured::constants::{JSON_FIDELITY, JSON_NULL, PATH_READ_KEYS};
use sqlquack_duckdb::semi_structured::main::json_get_computed::json_get_computed;
use sqlquack_duckdb::semi_structured::main::json_get_ignore_case::json_get_ignore_case;
use sqlquack_duckdb::semi_structured::main::json_guarded::json_guarded;
use sqlquack_duckdb::semi_structured::main::json_null_value::json_null_value;
use sqlquack_duckdb::semi_structured::main::json_path::json_path;
use sqlquack_duckdb::semi_structured::main::json_scalar::json_scalar;
use sqlquack_duckdb::semi_structured::main::json_serialized::json_serialized;
use sqlquack_duckdb::semi_structured::main::json_text::json_text;
use sqlquack_duckdb::semi_structured::main::json_zoned_wall::json_zoned_wall;
use sqlquack_duckdb::semi_structured::models::PathStep;
use sqlquack_duckdb::variants::main::json_kind_cast::json_kind_cast;
use sqlquack_duckdb::variants::main::json_type_name::json_type_name;
use sqlquack_snowflake::typing::models::{CastTarget, SnowflakeType};

use crate::conversion::_helpers::expressions::held::read_json;
use crate::conversion::_helpers::expressions::scalar::{convert_expression, convert_held};
use crate::conversion::_helpers::expressions::temporal::temporal_cast;
use crate::conversion::_helpers::support::divergence::{allows, diverge};
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal, VariantRoot};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, node_construct, node_kind, payload_object, require_understood,
};
use crate::conversion::constants::{
    IS_INTEGER_NAME, KIND_CAST_TARGETS, VARIANT_KIND_CAST, VARIANT_KIND_RULE,
};
use crate::conversion::constants::{
    LITERAL_KIND, NEGATE_KIND, PARSE_JSON_RULE, PARSE_JSON_VARIANT, SUBSCRIPT_PAIR,
    TRY_PARSE_JSON_NAME, VARIANT_CAST, VARIANT_COMPUTED_GET, VARIANT_HELD_UNREAD, VARIANT_INSPECT,
    VARIANT_NAME, VARIANT_PATH, VARIANT_TYPE_NAME,
};
use sqlquack_policy::policy::constants::{DATE_TEXT_SPELLING, TIMESTAMP_RANGE};

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
    let operand = convert_expression(text, site)?;
    let text = match operand.data_type {
        SnowflakeType::Varchar | SnowflakeType::Null => operand.sql,
        SnowflakeType::SemiStructured => {
            let _variant = site.context.log.rule(PARSE_JSON_VARIANT);
            json_text(&operand.sql, site.context.log)
        }
        other => {
            return Err(outside_conditions(
                construct.clone(),
                format!("a {other} operand needs an implicit conversion to text"),
            ));
        }
    };
    diverge(site.context, JSON_FIDELITY, construct)?;
    let trying = construct.name == TRY_PARSE_JSON_NAME;
    Ok(semi_structured(json_guarded(
        &text,
        trying,
        site.context.log,
    )))
}

/// A VARIANT column of a base table, guarded as PARSE_JSON's result is.
pub(crate) fn guarded_column(sql: &str, log: &ProvenanceLog) -> String {
    json_guarded(sql, false, log)
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
    let index = field(payload, "index", &construct)?;
    let key = convert_expression(index, site)?;
    match step_of(&key, &construct) {
        Ok(step) => located(&base, &[step], &construct, site.context),
        Err(refusal) if negative_literal(index) => Err(refusal),
        Err(refusal) => {
            let got = computed_get((base, key), refusal, &construct, site)?;
            site.context.log.equivalence(SUBSCRIPT_PAIR);
            Ok(got)
        }
    }
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
        "GET" if negative_literal(key) => Err(outside_conditions(
            construct.clone(),
            "GET at a negative constant index, which Snowflake rejects".to_owned(),
        )),
        "GET" => {
            let key = convert_expression(key, site)?;
            match step_of(&key, construct) {
                Ok(step) => located(&base, &[step], construct, site.context),
                Err(refusal) => computed_get((base, key), refusal, construct, site),
            }
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
            let base = json_only(base, construct)?;
            json_get_ignore_case(&base, &text, site.context.log)
                .map(semi_structured)
                .map_err(|reason| outside_conditions(construct.clone(), reason))
        }
    }
}

/// GET of `base` at a key or index `step` computed per row; `refusal` when it is another literal.
fn computed_get(
    (base, step): (Emitted, Emitted),
    refusal: Refusal,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    if step.literal.is_some() {
        return Err(refusal);
    }
    let index = match step.data_type {
        SnowflakeType::Number { scale: 0, .. } => true,
        SnowflakeType::Varchar => false,
        _ => return Err(refusal),
    };
    let _computed = site.context.log.rule(VARIANT_COMPUTED_GET);
    let base = json_only(base, construct)?;
    Ok(semi_structured(json_get_computed(
        &base,
        (&step.sql, index),
        site.context.log,
    )))
}

/// Whether `node` is a negated number literal, a constant Snowflake checks when it compiles.
fn negative_literal(node: &Value) -> bool {
    match node_kind(node) {
        Some((NEGATE_KIND, payload)) => payload
            .get("this")
            .and_then(node_kind)
            .is_some_and(|(kind, _)| kind == LITERAL_KIND),
        _ => false,
    }
}

/// TYPEOF, IS_NULL_VALUE, STRIP_NULL_VALUE and TO_JSON of a VARIANT.
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
    let held = base.is_variant_held();
    match name {
        "TYPEOF" => {
            let _rule = log.rule(VARIANT_TYPE_NAME);
            site.context
                .typed
                .inspect((base.data_type, base.untainted), construct);
            let typed = allows(site.context, VARIANT_TYPED_VALUES, construct);
            let sql = match held {
                true => variant_type_name(&base.sql, typed, log),
                false => json_type_name(&base.sql, typed, log),
            };
            Ok(Emitted::new(sql, SnowflakeType::Varchar))
        }
        "IS_DECIMAL" | "IS_INTEGER" => {
            let _rule = log.rule(VARIANT_KIND_RULE);
            if held {
                return Err(outside_conditions(
                    construct.clone(),
                    format!("{name} of a value DuckDB holds apart from JSON, such as a FLOAT"),
                ));
            }
            site.context
                .typed
                .inspect((base.data_type, base.untainted), construct);
            let types = match name == IS_INTEGER_NAME {
                true => "('INTEGER')",
                false => "('INTEGER', 'DECIMAL')",
            };
            Ok(Emitted::new(
                format!("({} IN {types})", json_type_name(&base.sql, false, log)),
                SnowflakeType::Boolean,
            ))
        }
        "IS_NULL_VALUE" => {
            let _rule = log.rule(VARIANT_INSPECT);
            Ok(Emitted::new(
                json_null_value(&read_json(&base, site.context), log),
                SnowflakeType::Boolean,
            ))
        }
        "STRIP_NULL_VALUE" => {
            let _rule = log.rule(VARIANT_INSPECT);
            let sql = format!(
                "(CASE WHEN {} THEN NULL ELSE {} END)",
                json_null_value(&read_json(&base, site.context), log),
                base.sql
            );
            Ok(Emitted {
                sql,
                literal: None,
                not_null: false,
                ..base
            })
        }
        _ => {
            let _rule = log.rule(VARIANT_INSPECT);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let sql = match held {
                true => variant_written(&base.sql, log),
                false => json_serialized(&base.sql, log),
            };
            Ok(Emitted::new(sql, SnowflakeType::Varchar))
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
        (SnowflakeType::TimestampTz { precision: 9 }, _) => {
            diverge(context, DATE_TEXT_SPELLING, construct).map_err(|refusal| refusal.reason)?;
            diverge(context, TIMESTAMP_RANGE, construct).map_err(|refusal| refusal.reason)?;
            temporal_cast(
                &json_zoned_wall(&operand.sql, context.log),
                (
                    SnowflakeType::TimestampNtz { precision: 9 },
                    target.data_type,
                ),
                (context, construct),
            )
        }
        (SnowflakeType::Varchar, Some(max_length)) => Ok(varchar_length_check(
            &json_text(&operand.sql, context.log),
            max_length,
            context.log,
        )),
        (SnowflakeType::Varchar, None) => Ok(json_text(&operand.sql, context.log)),
        (to, _) => json_scalar(&operand.sql, to, context.log),
    }
}

/// A cast of a VARIANT-held value: to VARIANT unchanged, to text or DATE through JSON, or FLOAT.
pub(crate) fn held_cast(
    operand: Emitted,
    (type_name, target): (&str, CastTarget),
    trying: bool,
    (context, construct): (&Context<'_>, &Construct),
) -> Result<Emitted, Refusal> {
    let log = context.log;
    let _rule = log.rule(VARIANT_CAST);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    if trying {
        return Err(refuse("TRY_CAST of a VARIANT has no proven conversion"));
    }
    if type_name == VARIANT_NAME {
        return Ok(operand);
    }
    if let Some((_, kind)) = KIND_CAST_TARGETS
        .iter()
        .find(|(name, _)| *name == type_name)
    {
        let _kind = log.rule(VARIANT_KIND_CAST);
        let checked = json_kind_cast(&variant_json(&operand.sql, log), kind, log);
        return Ok(Emitted {
            sql: format!(
                "(CASE WHEN {checked} IS NULL THEN NULL ELSE {} END)",
                operand.sql
            ),
            ..operand
        });
    }
    let sql = match (target.data_type, target.max_length) {
        (SnowflakeType::Varchar, max_length) => {
            diverge(context, JSON_FIDELITY, construct)?;
            if !matches!(operand.root, VariantRoot::Container | VariantRoot::Element) {
                diverge(context, FLOAT_TEXT, construct)?;
            }
            let text = variant_text(&operand.sql, operand.root == VariantRoot::Element, log);
            match max_length {
                Some(max_length) => varchar_length_check(&text, max_length, log),
                None => text,
            }
        }
        (SnowflakeType::Float, _) => {
            diverge(context, JSON_FIDELITY, construct)?;
            variant_double(&operand.sql, log)
        }
        (
            to @ (SnowflakeType::Date
            | SnowflakeType::Boolean
            | SnowflakeType::Number { scale: 0, .. }),
            _,
        ) => {
            diverge(context, JSON_FIDELITY, construct)?;
            json_scalar(&variant_json(&operand.sql, log), to, log)
                .map_err(|reason| refuse(&reason))?
        }
        (other, _) => {
            return Err(refuse(&format!(
                "a cast of a VARIANT built from a FLOAT to {other}, which no recording shows"
            )));
        }
    };
    Ok(Emitted::new(sql, target.data_type))
}

/// SQL NULL as a VARIANT.
pub(crate) fn variant_null(log: &ProvenanceLog) -> String {
    log.building_block(JSON_NULL);
    "CAST(NULL AS JSON)".to_owned()
}

fn semi_structured(sql: String) -> Emitted {
    Emitted::new(sql, SnowflakeType::SemiStructured)
}

fn variant_operand(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let emitted = convert_held(node, site)?;
    match emitted.data_type {
        SnowflakeType::SemiStructured => Ok(emitted),
        other => Err(outside_conditions(
            construct.clone(),
            format!("a path into a {other} value, not a VARIANT"),
        )),
    }
}

/// A literal path into a VARIANT, into a held one through its JSON.
fn located(
    base: &Emitted,
    steps: &[PathStep],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    let sql = json_path(&read_json(base, context), steps, context.log)
        .map_err(|reason| outside_conditions(construct.clone(), reason))?;
    Ok(match base.is_variant_held() {
        true => Emitted::variant_held(sql, VariantRoot::Element),
        false => semi_structured(sql),
    })
}

/// The JSON operand of a reader that has no rule for VARIANT-held values.
fn json_only(base: Emitted, construct: &Construct) -> Result<String, Refusal> {
    match base.is_variant_held() {
        true => Err(outside_conditions(
            construct.clone(),
            VARIANT_HELD_UNREAD.to_owned(),
        )),
        false => Ok(base.sql),
    }
}

/// The path step a converted key or index literal names.
fn step_of(key: &Emitted, construct: &Construct) -> Result<PathStep, Refusal> {
    match key.literal.clone() {
        Some(Literal::Text(key)) if PATH_READ_KEYS.contains(&key.as_str()) => {
            Err(outside_conditions(
                construct.clone(),
                format!("the key `{key}`, which a quoted JSON path cannot name"),
            ))
        }
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
