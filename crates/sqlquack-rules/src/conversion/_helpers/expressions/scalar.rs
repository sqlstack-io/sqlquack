//! Scalar expressions: dispatching each node kind to its rule.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;

use crate::conversion::_helpers::expressions::atoms::{
    boolean_literal, column, commented, literal, negate, null_literal, parentheses, scalar_subquery,
};
use crate::conversion::_helpers::expressions::calls::{convert_aggregate, convert_function};
use crate::conversion::_helpers::expressions::casts::convert_cast;
use crate::conversion::_helpers::expressions::conditionals::{case_expression, coalesce, iff, nvl};
use crate::conversion::_helpers::expressions::named_aggregates::{
    is_named_aggregate, named_aggregate,
};
use crate::conversion::_helpers::expressions::objects::object_constant;
use crate::conversion::_helpers::expressions::operators::{
    arithmetic, comparison_predicate, logic, modulo, negation, null_safe_comparison,
};
use crate::conversion::_helpers::expressions::ordered_aggregates::within_group;
use crate::conversion::_helpers::expressions::ordered_aggregates::{
    unordered_array_agg, unordered_array_agg_call,
};
use crate::conversion::_helpers::expressions::predicates::{
    between, exists, in_predicate, null_test,
};
use crate::conversion::_helpers::expressions::result_types::result_type;
use crate::conversion::_helpers::expressions::semi_structured::{variant_path, variant_subscript};
use crate::conversion::_helpers::expressions::spellings::{as_typed, as_written, rlike_as_call};
use crate::conversion::_helpers::expressions::strings::{
    case_mapping, concat_operator, text_affix, trim,
};
use crate::conversion::_helpers::expressions::temporal_functions::{date_diff, extract};
use crate::conversion::_helpers::expressions::text_functions::{length, like, substring};
use crate::conversion::_helpers::expressions::variants::variant_function;
use crate::conversion::_helpers::expressions::windows::convert_window;
use crate::conversion::_helpers::support::collecting::{
    collecting, events, placeholder, record, stopped,
};
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::shapes::{shape_argument, shape_construct};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    function_construct, missing, node_construct, node_kind, outside_conditions, payload_object,
    require_understood,
};
use crate::conversion::constants::{
    AGGREGATE_CALL_KIND, ARRAY_LITERAL_KIND, CALL_KIND, OBJECT_CONSTANT_KIND, RLIKE_PAIR,
    VARIANT_HELD_UNREAD,
};

/// One scalar expression, located; VARIANT-held values refused, collected refusals as placeholders.
pub(crate) fn convert_expression(node: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    converted(node, site, false)
}

/// One scalar expression, as [`convert_expression`], also when it is a VARIANT-held value.
pub(crate) fn convert_held(node: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    converted(node, site, true)
}

fn converted(node: &Value, site: Site<'_>, held: bool) -> Result<Emitted, Refusal> {
    let typed = site.context.typed;
    let reads = typed.reads();
    converted_checked(node, site, held).map(|emitted| {
        let (untainted, sourced) = typed.since(reads);
        Emitted {
            untainted,
            sourced,
            ..emitted
        }
    })
}

fn converted_checked(node: &Value, site: Site<'_>, held: bool) -> Result<Emitted, Refusal> {
    let checked = |node: &Value| {
        located(node, site).and_then(|emitted| match held || !emitted.is_variant_held() {
            true => Ok(emitted),
            false => Err(site.context.locator.placed(
                outside_conditions(held_construct(node), VARIANT_HELD_UNREAD.to_owned()),
                node,
            )),
        })
    };
    if !collecting() {
        return checked(node);
    }
    let before = events();
    match checked(node) {
        Ok(emitted) => Ok(emitted),
        Err(refusal) => {
            record(refusal, before);
            if matches!(node_kind(node), Some(("subquery" | "exists", _))) {
                stopped();
            }
            Ok(placeholder(result_type(node)))
        }
    }
}

/// One expression converted, its refusal placed at it and its raises identified.
fn located(node: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let locator = site.context.locator;
    let log = site.context.log;
    log.enter(&shape_construct(node), false);
    let converted = convert_kind(node, site);
    let argument = match &converted {
        Ok(emitted) => Some(shape_argument(node, emitted)),
        Err(_) => None,
    };
    log.leave(
        argument,
        (
            std::ptr::from_ref(node) as usize,
            locator.locate(node).map(|location| location.start),
        ),
    );
    let mut emitted = converted.map_err(|refusal| locator.placed(refusal, node))?;
    emitted.sql = locator.identified(&emitted.sql, node);
    Ok(emitted)
}

/// The construct a VARIANT-held value was built by, which a reader cannot take.
fn held_construct(node: &Value) -> Construct {
    match node_kind(node) {
        Some((kind, payload)) if kind == CALL_KIND || kind == AGGREGATE_CALL_KIND => {
            function_construct(&call_name(payload))
        }
        Some((kind, _)) => node_construct(kind),
        None => node_construct("unknown"),
    }
}

fn convert_kind(node: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let Some((kind, payload)) = node_kind(node) else {
        return Err(missing(
            node_construct("unknown"),
            "an expression node has an unexpected shape".to_owned(),
        ));
    };
    match kind {
        "column" => column(payload, site),
        "literal" => literal(payload, site),
        "boolean" => Ok(boolean_literal(payload, site)),
        "null" => Ok(null_literal(site)),
        "paren" => parentheses(payload, site),
        "annotated" => commented(payload, site),
        "neg" => negate(payload, site),
        "subquery" => scalar_subquery(payload, site),
        "add" | "sub" | "mul" | "div" => arithmetic(kind, payload, site),
        "mod" => modulo(payload, site),
        "eq" | "neq" | "lt" | "lte" | "gt" | "gte" => comparison_predicate(kind, payload, site),
        "null_safe_eq" | "null_safe_neq" => null_safe_comparison(kind, payload, site),
        "and" | "or" => logic(kind, payload, site),
        "not" => negation(payload, site),
        "is_null" => null_test(payload, site),
        "in" => in_predicate(payload, site),
        "between" => between(payload, site),
        "exists" => exists(payload, site),
        "case" => case_expression(payload, site),
        "coalesce" => coalesce(payload, site),
        "nvl" => nvl(payload, site),
        "if_func" => iff(payload, site),
        "cast" | "try_cast" => convert_cast(kind, payload, site),
        "upper" | "lower" => case_mapping(kind, payload, site),
        "trim" => trim(payload, site),
        "starts_with" | "ends_with" => text_affix(kind, payload, site),
        "substring" => substring(payload, site),
        "length" => length(payload, site),
        "like" | "i_like" => like(kind, payload, site),
        "extract" | "year" | "quarter" | "month" | "day" | "hour" | "minute" | "second" => {
            extract(kind, payload, site)
        }
        "date_diff" => date_diff(payload, site),
        "j_s_o_n_extract" => variant_path(payload, site),
        "subscript" => variant_subscript(payload, site),
        "concat" => concat_operator(payload, site),
        OBJECT_CONSTANT_KIND => object_constant(payload, site),
        ARRAY_LITERAL_KIND => array_literal(payload, site),
        "regexp_like" => match rlike_as_call(payload) {
            Some(call) => {
                site.context.log.equivalence(RLIKE_PAIR);
                convert_expression(&call, site)
            }
            None => Err(missing(
                node_construct(kind),
                "no rule handles RLIKE with flags".to_owned(),
            )),
        },
        "function" | "aggregate_function" => match as_typed(node) {
            Some((typed, spelling)) => {
                if let Some(pair) = spelling.pair {
                    site.context.log.equivalence(pair);
                }
                let arguments = payload.get("args").and_then(Value::as_array);
                let _adoption = site
                    .context
                    .locator
                    .adopt(&typed, arguments.map_or(&[], Vec::as_slice));
                convert_expression(&typed, site).map_err(|refusal| as_written(refusal, spelling))
            }
            None if kind == CALL_KIND => convert_function(payload, site),
            None if is_named_aggregate(&call_name(payload)) => {
                if !site.placement.aggregates {
                    return Err(outside_conditions(
                        function_construct(&call_name(payload)),
                        "an aggregate is not allowed here".to_owned(),
                    ));
                }
                named_aggregate(&call_name(payload), payload, site, None)
            }
            None if site.placement.aggregates && unordered_array_agg_call(node).is_some() => {
                let call = unordered_array_agg_call(node).ok_or_else(|| {
                    missing(
                        function_construct("ARRAY_AGG"),
                        "ARRAY_AGG has an unexpected shape".to_owned(),
                    )
                })?;
                unordered_array_agg(call, site)
            }
            None => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_uppercase();
                Err(missing(
                    function_construct(&name),
                    format!("no rule converts aggregate fn:{name}"),
                ))
            }
        },
        "count" | "sum" | "avg" | "min" | "max" => aggregate(kind, payload, site),
        "window_function" => window(payload, site),
        "within_group" if !site.placement.aggregates => Err(outside_conditions(
            node_construct(kind),
            "an aggregate is not allowed here".to_owned(),
        )),
        "within_group" => within_group(payload, site, None),
        other => Err(missing(
            node_construct(other),
            format!("no rule converts node:{other}"),
        )),
    }
}

/// A call's upper-case name.
pub(crate) fn call_name(payload: &Value) -> String {
    payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_uppercase()
}

fn aggregate(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    if !site.placement.aggregates {
        return Err(outside_conditions(
            node_construct(kind),
            "an aggregate is not allowed here".to_owned(),
        ));
    }
    convert_aggregate(kind, payload, site, None)
}

fn window(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    if !site.placement.windows {
        return Err(outside_conditions(
            node_construct("window_function"),
            "a window function is not allowed here".to_owned(),
        ));
    }
    convert_window(payload, site)
}

/// A bracket array literal, `[a, b]`, which Snowflake builds as ARRAY_CONSTRUCT(a, b) does.
fn array_literal(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(ARRAY_LITERAL_KIND);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["expressions", "bracket_notation", "use_list_keyword"],
        &construct,
    )?;
    let flag = |name: &str| payload.get(name).and_then(Value::as_bool).unwrap_or(false);
    if !flag("bracket_notation") || flag("use_list_keyword") {
        return Err(outside_conditions(
            construct,
            "an ARRAY or LIST constructor other than a bracket literal".to_owned(),
        ));
    }
    let elements = payload
        .get("expressions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    variant_function("ARRAY_CONSTRUCT", &elements, &construct, site)
}
