//! OBJECT_CONSTRUCT, OBJECT_INSERT and OBJECT_DELETE over VARIANT values held as DuckDB `JSON`.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::semi_structured::constants::{JSON_FIDELITY, VARIANT_TYPED_VALUES};
use sqlquack_duckdb::semi_structured::main::json_guarded::json_guarded;
use sqlquack_duckdb::semi_structured::models::TypedSource;
use sqlquack_duckdb::variants::main::json_object_construct::json_object_construct;
use sqlquack_duckdb::variants::main::json_object_delete::json_object_delete;
use sqlquack_duckdb::variants::main::json_object_insert::json_object_insert;
use sqlquack_duckdb::variants::main::json_typed_text::json_typed_text;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    KEEP_NULL_CONSTRUCTOR, MICROSECOND_DIGITS, OBJECT_CONSTANT_KIND, OBJECT_CONSTRUCT_RULE,
    OBJECT_CONSTRUCTOR, OBJECT_DELETE_RULE, OBJECT_INSERT_RULE,
};

/// OBJECT_DELETE of a VARIANT and literal text keys.
pub(crate) fn object_delete(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(OBJECT_DELETE_RULE);
    let [object, keys @ ..] = arguments else {
        return Err(refused(construct, "OBJECT_DELETE without an object"));
    };
    if keys.is_empty() {
        return Err(refused(construct, "OBJECT_DELETE without a key"));
    }
    let object = variant(object, construct, site)?;
    let keys = keys
        .iter()
        .map(|key| text_literal(key, construct, site))
        .collect::<Result<Vec<String>, Refusal>>()?;
    Ok(semi_structured(json_object_delete(
        &object,
        &keys,
        site.context.log,
    )))
}

/// OBJECT_INSERT of a VARIANT, a literal key, a value and an optional literal update flag.
pub(crate) fn object_insert(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(OBJECT_INSERT_RULE);
    let (object, key, value, update) = match arguments {
        [object, key, value] => (object, key, value, false),
        [object, key, value, update] => (
            object,
            key,
            value,
            boolean_literal(update, construct, site)?,
        ),
        _ => {
            return Err(refused(
                construct,
                "OBJECT_INSERT takes three or four arguments",
            ));
        }
    };
    let object = variant(object, construct, site)?;
    let key = text_literal(key, construct, site)?;
    let value = json_value(value, construct, site)?;
    Ok(semi_structured(json_object_insert(
        (&object, &value),
        (&key, update),
        log,
    )))
}

/// OBJECT_CONSTRUCT or OBJECT_CONSTRUCT_KEEP_NULL of text keys and values.
pub(crate) fn object_construct(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(OBJECT_CONSTRUCT_RULE);
    if !arguments.len().is_multiple_of(2) {
        return Err(refused(
            construct,
            "OBJECT_CONSTRUCT of other than key-value pairs",
        ));
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    for pair in arguments.chunks(2) {
        let [key, value] = pair else {
            return Err(refused(
                construct,
                "OBJECT_CONSTRUCT of other than key-value pairs",
            ));
        };
        let key = convert_expression(key, site)?;
        if !matches!(key.data_type, SnowflakeType::Varchar | SnowflakeType::Null) {
            return Err(refused(
                construct,
                "an OBJECT_CONSTRUCT key that is not text",
            ));
        }
        pairs.push((key.sql, json_value(value, construct, site)?));
    }
    Ok(semi_structured(json_object_construct(
        &pairs,
        name == KEEP_NULL_CONSTRUCTOR,
        log,
    )))
}

/// An object constant `{'key': value, ...}`, as OBJECT_CONSTRUCT of its pairs.
pub(crate) fn object_constant(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(OBJECT_CONSTANT_KIND);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["keys", "values", "curly_brace_syntax", "with_map_keyword"],
        &construct,
    )?;
    let flag = |name: &str| payload.get(name).and_then(Value::as_bool).unwrap_or(false);
    if !flag("curly_brace_syntax") || flag("with_map_keyword") {
        return Err(refused(&construct, "a MAP constructor"));
    }
    let list = |name: &str| {
        payload
            .get(name)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let (keys, values) = (list("keys"), list("values"));
    if keys.len() != values.len() {
        return Err(refused(&construct, "an object constant of unpaired keys"));
    }
    let arguments: Vec<Value> = keys
        .into_iter()
        .zip(values)
        .flat_map(|(key, value)| [key, value])
        .collect();
    object_construct(OBJECT_CONSTRUCTOR, &arguments, &construct, site)
}

/// A value as JSON: a VARIANT as it is; text, booleans and whole numbers as Snowflake stores them.
pub(crate) fn json_value(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    as_json(convert_expression(node, site)?, construct, site.context)
}

/// A converted value as JSON, as [`json_value`] describes.
pub(crate) fn as_json(
    emitted: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let sql = emitted.sql;
    match emitted.data_type {
        SnowflakeType::SemiStructured => Ok(sql),
        SnowflakeType::Null => Ok("CAST(NULL AS JSON)".to_owned()),
        SnowflakeType::Varchar | SnowflakeType::Boolean => Ok(format!("to_json({sql})")),
        SnowflakeType::Number { scale: 0, .. } => {
            diverge(context, JSON_FIDELITY, construct)?;
            Ok(json_guarded(
                &format!("CAST({sql} AS VARCHAR)"),
                context.log,
            ))
        }
        typed @ (SnowflakeType::Date
        | SnowflakeType::TimestampNtz { .. }
        | SnowflakeType::TimestampLtz { .. }) => {
            diverge(context, VARIANT_TYPED_VALUES, construct)?;
            let source = match typed {
                SnowflakeType::Date => TypedSource::Date,
                SnowflakeType::TimestampNtz { precision } if precision > MICROSECOND_DIGITS => {
                    TypedSource::TimestampNanos
                }
                SnowflakeType::TimestampNtz { .. } => TypedSource::Timestamp,
                _ => TypedSource::TimestampLocal,
            };
            Ok(json_typed_text(&sql, source, context.log))
        }
        other => Err(refused(
            construct,
            &format!("a {other} value in a VARIANT, which Snowflake keeps typed"),
        )),
    }
}

pub(crate) fn variant(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::SemiStructured => Ok(emitted.sql),
        other => Err(refused(
            construct,
            &format!("an object argument of type {other}"),
        )),
    }
}

pub(crate) fn text_literal(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    match convert_expression(node, site)?.literal {
        Some(Literal::Text(text)) => Ok(text),
        _ => Err(refused(construct, "a key other than a text literal")),
    }
}

fn boolean_literal(node: &Value, construct: &Construct, site: Site<'_>) -> Result<bool, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.sql.to_ascii_uppercase().as_str() {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(refused(
            construct,
            "an update flag other than TRUE or FALSE",
        )),
    }
}

pub(crate) fn semi_structured(sql: String) -> Emitted {
    Emitted::new(sql, SnowflakeType::SemiStructured)
}

pub(crate) fn refused(construct: &Construct, reason: &str) -> Refusal {
    outside_conditions(construct.clone(), reason.to_owned())
}
