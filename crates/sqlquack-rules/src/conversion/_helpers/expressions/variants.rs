//! OBJECT_PICK, OBJECT_KEYS, ARRAY_ functions, SPLIT, TO_VARIANT and IS_OBJECT-style checks.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::semi_structured::constants::{JSON_FIDELITY, VARIANT_TYPED_VALUES};
use sqlquack_duckdb::semi_structured::main::json_array_to_string::json_array_to_string;
use sqlquack_duckdb::semi_structured::models::PickedKeys;
use sqlquack_duckdb::variants::main::json_array_cat::json_array_cat;
use sqlquack_duckdb::variants::main::json_array_compact::json_array_compact;
use sqlquack_duckdb::variants::main::json_array_construct::json_array_construct;
use sqlquack_duckdb::variants::main::json_array_construct_compact::json_array_construct_compact;
use sqlquack_duckdb::variants::main::json_array_contains::json_array_contains;
use sqlquack_duckdb::variants::main::json_array_except::json_array_except;
use sqlquack_duckdb::variants::main::json_array_position::json_array_position;
use sqlquack_duckdb::variants::main::json_array_size::json_array_size;
use sqlquack_duckdb::variants::main::json_is_kind::json_is_kind;
use sqlquack_duckdb::variants::main::json_is_typed::json_is_typed;
use sqlquack_duckdb::variants::main::json_object_keys::json_object_keys;
use sqlquack_duckdb::variants::main::json_object_pick::json_object_pick;
use sqlquack_duckdb::variants::main::json_split::json_split;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::objects::{
    json_value, refused, semi_structured, text_literal, variant,
};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::support::divergence::{allows, diverge};
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::constants::{
    ARRAY_CAT_RULE, ARRAY_COMPACT_RULE, ARRAY_CONSTRUCT_COMPACT_RULE, ARRAY_CONSTRUCT_RULE,
    ARRAY_CONTAINS_RULE, ARRAY_EXCEPT_RULE, ARRAY_POSITION_RULE, ARRAY_POSITION_TYPE,
    ARRAY_SIZE_RULE, ARRAY_TO_STRING_RULE, OBJECT_KEYS_RULE, OBJECT_PICK_RULE, SPLIT_RULE,
    TO_VARIANT_RULE, TYPED_CHECKS, VARIANT_KIND_RULE, VARIANT_KINDS, VARIANT_TYPED_CHECK,
};

/// A VARIANT operand, or a NULL literal as a NULL VARIANT.
fn variant_or_null(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::SemiStructured => Ok(emitted.sql),
        SnowflakeType::Null => Ok("CAST(NULL AS JSON)".to_owned()),
        other => Err(refused(construct, &format!("a needle of type {other}"))),
    }
}

/// One of the semi-structured functions this module converts, by its upper-case name.
pub(crate) fn variant_function(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    match (name, arguments) {
        ("OBJECT_KEYS", [object]) => {
            let _rule = log.rule(OBJECT_KEYS_RULE);
            let object = variant(object, construct, site)?;
            Ok(semi_structured(json_object_keys(&object, log)))
        }
        ("OBJECT_PICK", [object, keys @ ..]) if !keys.is_empty() => {
            object_pick(object, keys, construct, site)
        }
        ("ARRAY_CONSTRUCT", values) => {
            let _rule = log.rule(ARRAY_CONSTRUCT_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let values = values
                .iter()
                .map(|value| json_value(value, construct, site))
                .collect::<Result<Vec<String>, Refusal>>()?;
            Ok(semi_structured(json_array_construct(&values, log)))
        }
        ("ARRAY_CONSTRUCT_COMPACT", values) => {
            let _rule = log.rule(ARRAY_CONSTRUCT_COMPACT_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let values = values
                .iter()
                .map(|value| json_value(value, construct, site))
                .collect::<Result<Vec<String>, Refusal>>()?;
            Ok(semi_structured(json_array_construct_compact(&values, log)))
        }
        ("ARRAY_EXCEPT", [left, right]) => {
            let _rule = log.rule(ARRAY_EXCEPT_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let left = variant(left, construct, site)?;
            let right = variant(right, construct, site)?;
            Ok(semi_structured(json_array_except((&left, &right), log)))
        }
        ("ARRAY_POSITION", [needle, array]) => {
            let _rule = log.rule(ARRAY_POSITION_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let needle = variant_or_null(needle, construct, site)?;
            let array = variant(array, construct, site)?;
            Ok(Emitted::new(
                cast(
                    &json_array_position((&needle, &array), log),
                    ARRAY_POSITION_TYPE,
                    log,
                )
                .map_err(|reason| refused(construct, &reason))?,
                ARRAY_POSITION_TYPE,
            ))
        }
        ("ARRAY_CAT", [left, right]) => {
            let _rule = log.rule(ARRAY_CAT_RULE);
            let left = variant(left, construct, site)?;
            let right = variant(right, construct, site)?;
            Ok(semi_structured(json_array_cat((&left, &right), log)))
        }
        ("ARRAY_COMPACT", [array]) => {
            let _rule = log.rule(ARRAY_COMPACT_RULE);
            let array = variant(array, construct, site)?;
            Ok(semi_structured(json_array_compact(&array, log)))
        }
        ("ARRAY_SIZE", [array]) => {
            let _rule = log.rule(ARRAY_SIZE_RULE);
            let array = variant(array, construct, site)?;
            Ok(Emitted::new(
                json_array_size(&array, log),
                SnowflakeType::Number {
                    precision: 9,
                    scale: 0,
                },
            ))
        }
        ("ARRAY_CONTAINS", [value, array]) => {
            let _rule = log.rule(ARRAY_CONTAINS_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let value = variant(value, construct, site)?;
            let array = variant(array, construct, site)?;
            Ok(Emitted::new(
                json_array_contains((&value, &array), log),
                SnowflakeType::Boolean,
            ))
        }
        ("ARRAY_TO_STRING", [array, separator]) => {
            let _rule = log.rule(ARRAY_TO_STRING_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let array = variant(array, construct, site)?;
            let separator = text_operand(separator, construct, site)?;
            Ok(Emitted::new(
                json_array_to_string((&array, &separator), log),
                SnowflakeType::Varchar,
            ))
        }
        ("SPLIT", [text, separator]) => {
            let _rule = log.rule(SPLIT_RULE);
            let text = text_operand(text, construct, site)?;
            let separator = text_operand(separator, construct, site)?;
            Ok(semi_structured(json_split((&text, &separator), log)))
        }
        ("TO_VARIANT", [value]) => {
            let _rule = log.rule(TO_VARIANT_RULE);
            Ok(semi_structured(json_value(value, construct, site)?))
        }
        (_, [value]) if TYPED_CHECKS.contains(&name) => {
            let _rule = log.rule(VARIANT_TYPED_CHECK);
            let value = variant(value, construct, site)?;
            let sql = match allows(site.context, VARIANT_TYPED_VALUES, construct) {
                true => json_is_typed(&value, log),
                false => format!("(CASE WHEN {value} IS NULL THEN NULL ELSE false END)"),
            };
            Ok(Emitted::new(sql, SnowflakeType::Boolean))
        }
        (_, [value]) => match VARIANT_KINDS.iter().find(|(function, _)| *function == name) {
            Some((_, kind)) => {
                let _rule = log.rule(VARIANT_KIND_RULE);
                let value = variant(value, construct, site)?;
                Ok(Emitted::new(
                    json_is_kind(&value, kind, log),
                    SnowflakeType::Boolean,
                ))
            }
            None => Err(refused(construct, &format!("{name} of one argument"))),
        },
        _ => Err(refused(
            construct,
            &format!("{name} with {} arguments", arguments.len()),
        )),
    }
}

/// OBJECT_PICK of literal text keys, or of one VARIANT array of strings.
fn object_pick(
    object: &Value,
    keys: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(OBJECT_PICK_RULE);
    let object = variant(object, construct, site)?;
    let picked = match keys {
        [array] if !matches!(literal(array, site)?, Some(Literal::Text(_))) => {
            diverge(site.context, JSON_FIDELITY, construct)?;
            PickedKeys::Array(variant(array, construct, site)?)
        }
        _ => PickedKeys::Literal(
            keys.iter()
                .map(|key| text_literal(key, construct, site))
                .collect::<Result<Vec<String>, Refusal>>()?,
        ),
    };
    Ok(semi_structured(json_object_pick(&object, &picked, log)))
}

fn literal(node: &Value, site: Site<'_>) -> Result<Option<Literal>, Refusal> {
    Ok(convert_expression(node, site)?.literal)
}
