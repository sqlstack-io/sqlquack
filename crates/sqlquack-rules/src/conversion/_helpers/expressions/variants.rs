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

use crate::conversion::_helpers::expressions::arrays::array_function;
use crate::conversion::_helpers::expressions::held::{built, leaves, read_json};
use crate::conversion::_helpers::expressions::objects::{
    free_variant, refused, semi_structured, text_literal, variant,
};
use crate::conversion::_helpers::expressions::ordered_aggregates::{
    sorted_array_agg, unordered_array_agg_call,
};
use crate::conversion::_helpers::expressions::scalar::{
    call_name, convert_expression, convert_held,
};
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::support::divergence::{allows, diverge};
use crate::conversion::_helpers::support::scope::{Context, Emitted, Literal, VariantRoot};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, flag, function_construct, list, node_kind, require_understood,
};
use crate::conversion::_helpers::support::typed_leaves::compared_by_text;
use crate::conversion::constants::DUPLICATE_VARIANTS;
use crate::conversion::constants::{
    ARRAY_AGG_RULE, ARRAY_APPEND_RULE, ARRAY_CAT_RULE, ARRAY_COMPACT_RULE,
    ARRAY_CONSTRUCT_COMPACT_RULE, ARRAY_CONSTRUCT_RULE, ARRAY_CONTAINS_RULE, ARRAY_EXCEPT_RULE,
    ARRAY_POSITION_RULE, ARRAY_POSITION_TYPE, ARRAY_SIZE_RULE, ARRAY_TO_STRING_RULE,
    COUNTED_ARRAY_FUNCTIONS, FUNCTION_KIND, OBJECT_KEYS_RULE, OBJECT_PICK_RULE,
    PICKED_ARRAY_FUNCTIONS, SORTED_ARRAY_FUNCTION, SPLIT_RULE, TO_VARIANT_RULE, TYPED_CHECKS,
    VARIANT_KIND_RULE, VARIANT_KINDS, VARIANT_TYPED_CHECK,
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

/// ARRAY_CONSTRUCT, or with `compact` ARRAY_CONSTRUCT_COMPACT, of `values`, held when typed.
pub(crate) fn array_construct(
    values: &[Value],
    compact: bool,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(match compact {
        true => ARRAY_CONSTRUCT_COMPACT_RULE,
        false => ARRAY_CONSTRUCT_RULE,
    });
    diverge(site.context, JSON_FIDELITY, construct)?;
    let values = values
        .iter()
        .map(|value| convert_held(value, site))
        .collect::<Result<Vec<Emitted>, Refusal>>()?;
    let (values, held) = leaves(values, construct, site.context)?;
    let sql = match compact {
        true => json_array_construct_compact(&values, log),
        false => json_array_construct(&values, false, log),
    };
    Ok(built(sql, held, VariantRoot::Container, site.context))
}

/// ARRAY_SIZE of an unordered ARRAY_AGG `call`: the count of its non-NULL values.
fn array_agg_size(
    call: &Payload,
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(ARRAY_SIZE_RULE);
    let _aggregate = log.rule(ARRAY_AGG_RULE);
    let [value] = list(call, "args") else {
        return Err(refused(construct, "ARRAY_AGG takes one value"));
    };
    let value = convert_expression(value, site.placed(Placement::ROW))?;
    if flag(call, "distinct") && value.data_type == SnowflakeType::SemiStructured {
        return Err(refused(
            construct,
            &format!("ARRAY_AGG DISTINCT {DUPLICATE_VARIANTS}"),
        ));
    }
    let distinct = match flag(call, "distinct") {
        true => "DISTINCT ",
        false => "",
    };
    let data_type = SnowflakeType::Number {
        precision: 9,
        scale: 0,
    };
    let sql = cast(&format!("count({distinct}{})", value.sql), data_type, log)
        .map_err(|reason| refused(construct, &reason))?;
    Ok(Emitted::new(sql, data_type))
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
        ("ARRAY_CONSTRUCT", values) => array_construct(values, false, construct, site),
        ("ARRAY_CONSTRUCT_COMPACT", values) => array_construct(values, true, construct, site),
        ("ARRAY_EXCEPT", [left, right]) => {
            let _rule = log.rule(ARRAY_EXCEPT_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let reads = site.context.typed.reads();
            let (left, left_unordered) = free_variant(left, construct, site)?;
            let (right, right_unordered) = free_variant(right, construct, site)?;
            compared_by_text(site.context.typed.since(reads), construct, site.context)?;
            Ok(Emitted {
                unordered: left_unordered || right_unordered,
                ..semi_structured(json_array_except((&left, &right), log))
            })
        }
        ("ARRAY_POSITION", [needle, array]) => {
            let _rule = log.rule(ARRAY_POSITION_RULE);
            diverge(site.context, JSON_FIDELITY, construct)?;
            let reads = site.context.typed.reads();
            let needle = variant_or_null(needle, construct, site)?;
            let array = variant(array, construct, site)?;
            compared_by_text(site.context.typed.since(reads), construct, site.context)?;
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
            let (left, left_unordered) = free_variant(left, construct, site)?;
            let (right, right_unordered) = free_variant(right, construct, site)?;
            Ok(Emitted {
                unordered: left_unordered || right_unordered,
                ..semi_structured(json_array_cat((&left, &right), log))
            })
        }
        ("ARRAY_COMPACT", [array]) => {
            let _rule = log.rule(ARRAY_COMPACT_RULE);
            let array = variant(array, construct, site)?;
            Ok(semi_structured(json_array_compact(&array, log)))
        }
        ("ARRAY_SIZE", [array])
            if site.placement.aggregates && unordered_array_agg_call(array).is_some() =>
        {
            let call = unordered_array_agg_call(array)
                .ok_or_else(|| refused(construct, "ARRAY_SIZE of other than ARRAY_AGG"))?;
            array_agg_size(call, construct, site)
        }
        ("ARRAY_SORT", [array])
            if site.placement.aggregates && unordered_array_agg_call(array).is_some() =>
        {
            let call = unordered_array_agg_call(array)
                .ok_or_else(|| refused(construct, "ARRAY_SORT of other than ARRAY_AGG"))?;
            sorted_array_agg(call, site)
        }
        ("ARRAY_SIZE", [array]) => {
            let _rule = log.rule(ARRAY_SIZE_RULE);
            let mark = site.context.order.mark();
            let array = counted_array(array, construct, site.order_free(true))?;
            site.context.order.absorb(mark);
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
            let reads = site.context.typed.reads();
            let value = variant(value, construct, site)?;
            let (array, _) = free_variant(array, construct, site)?;
            compared_by_text(site.context.typed.since(reads), construct, site.context)?;
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
            to_variant(convert_held(value, site)?, construct, site.context)
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
        (SORTED_ARRAY_FUNCTION, [_]) => array_function(name, arguments, construct, site)
            .unwrap_or_else(|| Err(refused(construct, &format!("{name} of one argument")))),
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

/// OBJECT_PICK of literal text keys, or of one array of strings an array builder makes.
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
            if !built_array(array) {
                return Err(refused(
                    construct,
                    "OBJECT_PICK of keys in a VARIANT, which Snowflake does not pick by its elements",
                ));
            }
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

/// Whether `node` calls a function recorded giving OBJECT_PICK an ARRAY rather than a VARIANT.
fn built_array(node: &Value) -> bool {
    node_kind(node).is_some_and(|(kind, payload)| {
        kind == FUNCTION_KIND && PICKED_ARRAY_FUNCTIONS.contains(&call_name(payload).as_str())
    })
}

fn literal(node: &Value, site: Site<'_>) -> Result<Option<Literal>, Refusal> {
    Ok(convert_expression(node, site)?.literal)
}

/// TO_VARIANT or `::VARIANT` of a converted value: a FLOAT is held as a DuckDB VARIANT.
pub(crate) fn to_variant(
    value: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    if value.is_variant_held() {
        return Ok(value);
    }
    let root = match value.data_type {
        SnowflakeType::Float => VariantRoot::Double,
        _ => VariantRoot::Dynamic,
    };
    let (values, held) = leaves(vec![value], construct, context)?;
    let sql = values.into_iter().next().unwrap_or_default();
    Ok(built(sql, held, root, context))
}

/// ARRAY_SIZE's operand, ARRAY_CONSTRUCT, _APPEND and _CAT counting undefined elements as Snowflake.
fn counted_array(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    let Some(call) = node.get(FUNCTION_KIND).and_then(Value::as_object) else {
        return held_variant(node, construct, site);
    };
    let name = call
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase();
    let shadowed = site.context.schema.functions.contains_key(&name)
        || site.context.options.overrides.contains_key(&name);
    if shadowed || !COUNTED_ARRAY_FUNCTIONS.contains(&name.as_str()) {
        return held_variant(node, construct, site);
    }
    let called = function_construct(&name);
    require_understood(call, &["name", "args"], &called)?;
    let log = site.context.log;
    match (name.as_str(), list(call, "args")) {
        ("ARRAY_CONSTRUCT", values) => {
            let _rule = log.rule(ARRAY_CONSTRUCT_RULE);
            counted_elements(values, &called, site)
        }
        ("ARRAY_APPEND", [array, value]) => {
            let _rule = log.rule(ARRAY_APPEND_RULE);
            let array = counted_array(array, construct, site)?;
            let appended = counted_elements(std::slice::from_ref(value), &called, site)?;
            Ok(json_array_cat((&array, &appended), log))
        }
        ("ARRAY_CAT", [left, right]) => {
            let _rule = log.rule(ARRAY_CAT_RULE);
            let left = counted_array(left, construct, site)?;
            let right = counted_array(right, construct, site)?;
            Ok(json_array_cat((&left, &right), log))
        }
        _ => held_variant(node, construct, site),
    }
}

/// The values as a JSON array whose length alone is read; a FLOAT held apart refuses.
fn counted_elements(
    values: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    diverge(site.context, JSON_FIDELITY, construct)?;
    let values = values
        .iter()
        .map(|value| convert_held(value, site))
        .collect::<Result<Vec<Emitted>, Refusal>>()?;
    let (values, held) = leaves(values, construct, site.context)?;
    if held {
        return Err(refused(
            construct,
            "an array element of a FLOAT, which DuckDB holds apart from JSON",
        ));
    }
    Ok(json_array_construct(&values, true, site.context.log))
}

/// A VARIANT argument read as JSON, also when it is VARIANT-held.
fn held_variant(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    let emitted = convert_held(node, site)?;
    match emitted.data_type {
        SnowflakeType::SemiStructured => Ok(read_json(&emitted, site.context)),
        other => Err(refused(
            construct,
            &format!("an object argument of type {other}"),
        )),
    }
}
