//! Scalar expressions: dispatching each node kind to its rule.

use serde_json::Value;
use sqlquack_core::refusals::models::Refusal;

use crate::conversion::_helpers::expressions::atoms::{
    boolean_literal, column, commented, literal, negate, null_literal, parentheses, scalar_subquery,
};
use crate::conversion::_helpers::expressions::calls::{convert_aggregate, convert_function};
use crate::conversion::_helpers::expressions::casts::convert_cast;
use crate::conversion::_helpers::expressions::conditionals::{case_expression, coalesce, iff, nvl};
use crate::conversion::_helpers::expressions::operators::{
    arithmetic, comparison_predicate, logic, negation, null_safe_comparison,
};
use crate::conversion::_helpers::expressions::ordered_aggregates::within_group;
use crate::conversion::_helpers::expressions::predicates::{
    between, exists, in_predicate, null_test,
};
use crate::conversion::_helpers::expressions::semi_structured::{variant_path, variant_subscript};
use crate::conversion::_helpers::expressions::spellings::{as_typed, as_written, rlike_as_call};
use crate::conversion::_helpers::expressions::strings::{
    case_mapping, concat_operator, text_affix, trim,
};
use crate::conversion::_helpers::expressions::temporal_functions::{date_diff, extract};
use crate::conversion::_helpers::expressions::text_functions::{length, like, substring};
use crate::conversion::_helpers::expressions::windows::convert_window;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    function_construct, missing, node_construct, node_kind, outside_conditions,
};
use crate::conversion::constants::{CALL_KIND, RLIKE_PAIR};

/// Converts one scalar expression; refusals and raises inside it name their innermost node.
pub(crate) fn convert_expression(node: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let locator = site.context.locator;
    let mut emitted = convert_kind(node, site).map_err(|refusal| locator.placed(refusal, node))?;
    emitted.sql = locator.identified(&emitted.sql, node);
    Ok(emitted)
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
