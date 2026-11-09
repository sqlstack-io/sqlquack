//! Sorting, flattening, set, slicing and range functions over VARIANT arrays held as JSON.

use serde_json::Value;
use sqlquack_blocks::arrays::constants::ARRAY_SORT_ORDER;
use sqlquack_blocks::arrays::main::json_array_distinct::json_array_distinct;
use sqlquack_blocks::arrays::main::json_array_extreme::json_array_extreme;
use sqlquack_blocks::arrays::main::json_array_flatten::json_array_flatten;
use sqlquack_blocks::arrays::main::json_array_intersection::json_array_intersection;
use sqlquack_blocks::arrays::main::json_array_range::json_array_range;
use sqlquack_blocks::arrays::main::json_array_remove::json_array_remove;
use sqlquack_blocks::arrays::main::json_array_reverse::json_array_reverse;
use sqlquack_blocks::arrays::main::json_array_slice::json_array_slice;
use sqlquack_blocks::arrays::main::json_array_sort::json_array_sort;
use sqlquack_blocks::arrays::main::json_arrays_overlap::json_arrays_overlap;
use sqlquack_blocks::arrays::main::json_to_array::json_to_array;
use sqlquack_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_blocks::variants::main::json_array_cat::json_array_cat;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::objects::{
    free_variant, refused, semi_structured, variant,
};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::text_functions::integer_operand;
use crate::conversion::_helpers::expressions::variants::array_construct;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::typed_leaves::compared_by_text;
use crate::conversion::constants::{
    ARRAY_APPEND_RULE, ARRAY_EXTREME_RULE, ARRAY_FLATTEN_RULE, ARRAY_MAX_NAME, ARRAY_RANGE_RULE,
    ARRAY_REVERSE_RULE, ARRAY_SET_RULE, ARRAY_SLICE_RULE, ARRAY_SORT_RULE, TO_ARRAY_RULE,
};

/// One of this module's array functions, or `None` for another name or arity.
pub(crate) fn array_function(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Option<Result<Emitted, Refusal>> {
    let log = site.context.log;
    Some(match (name, arguments) {
        ("ARRAY_SORT", [array]) => {
            let _rule = log.rule(ARRAY_SORT_RULE);
            diverge(site.context, ARRAY_SORT_ORDER, construct)
                .and_then(|()| free_variant(array, construct, site))
                .map(|(array, _)| semi_structured(json_array_sort(&array, log)))
        }
        ("ARRAY_FLATTEN", [array]) => {
            let _rule = log.rule(ARRAY_FLATTEN_RULE);
            variant(array, construct, site)
                .map(|array| semi_structured(json_array_flatten(&array, log)))
        }
        ("TO_ARRAY", [value]) => to_array(value, construct, site),
        ("ARRAY_REVERSE", [array]) => {
            let _rule = log.rule(ARRAY_REVERSE_RULE);
            variant(array, construct, site)
                .map(|array| semi_structured(json_array_reverse(&array, log)))
        }
        ("ARRAY_MAX" | "ARRAY_MIN", [array]) => {
            let _rule = log.rule(ARRAY_EXTREME_RULE);
            diverge(site.context, ARRAY_SORT_ORDER, construct)
                .and_then(|()| free_variant(array, construct, site))
                .map(|(array, _)| {
                    semi_structured(json_array_extreme(&array, name == ARRAY_MAX_NAME, log))
                })
        }
        ("ARRAY_INTERSECTION" | "ARRAYS_OVERLAP", [left, right]) => {
            compared_arrays(name, (left, Some(right)), construct, site)
        }
        ("ARRAY_DISTINCT", [array]) => compared_arrays(name, (array, None), construct, site),
        ("ARRAY_REMOVE", [array, value]) => array_remove((array, value), construct, site),
        ("ARRAY_APPEND", [array, value]) => array_append((array, value), construct, site),
        ("ARRAY_SLICE", [array, from, to]) => {
            let _rule = log.rule(ARRAY_SLICE_RULE);
            array_slice((array, from, to), construct, site)
        }
        ("ARRAY_GENERATE_RANGE", [start, end, step @ ..]) if step.len() <= 1 => {
            let _rule = log.rule(ARRAY_RANGE_RULE);
            array_range((start, end, step.first()), construct, site)
        }
        _ => return None,
    })
}

/// TO_ARRAY: a VARIANT array kept, NULL and JSON null as NULL, another value in a one-element array.
fn to_array(value: &Value, construct: &Construct, site: Site<'_>) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(TO_ARRAY_RULE);
    let emitted = convert_expression(value, site)?;
    if emitted.data_type == SnowflakeType::SemiStructured {
        return Ok(semi_structured(json_to_array(&emitted.sql, log)));
    }
    let wrapped = array_construct(std::slice::from_ref(value), false, construct, site)?;
    if wrapped.is_variant_held() {
        return Err(refused(
            construct,
            "TO_ARRAY of a FLOAT, which DuckDB holds apart from JSON",
        ));
    }
    Ok(semi_structured(format!(
        "(CASE WHEN {} IS NULL THEN NULL ELSE {} END)",
        emitted.sql, wrapped.sql
    )))
}

/// ARRAY_INTERSECTION, ARRAYS_OVERLAP and ARRAY_DISTINCT, elements equal where Snowflake writes them alike.
fn compared_arrays(
    name: &str,
    (left, right): (&Value, Option<&Value>),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(ARRAY_SET_RULE);
    diverge(site.context, JSON_FIDELITY, construct)?;
    let reads = site.context.typed.reads();
    let (left, left_unordered) = free_variant(left, construct, site)?;
    let right = right
        .map(|right| free_variant(right, construct, site))
        .transpose()?;
    compared_by_text(site.context.typed.since(reads), construct, site.context)?;
    let unordered = left_unordered || right.as_ref().is_some_and(|(_, unordered)| *unordered);
    Ok(match (name, right) {
        ("ARRAY_INTERSECTION", Some((right, _))) => Emitted {
            unordered,
            ..semi_structured(json_array_intersection((&left, &right), log))
        },
        (_, Some((right, _))) => Emitted::new(
            json_arrays_overlap((&left, &right), log),
            SnowflakeType::Boolean,
        ),
        (_, None) => Emitted {
            unordered,
            ..semi_structured(json_array_distinct(&left, log))
        },
    })
}

/// ARRAY_REMOVE of a VARIANT value: elements equal to it as Snowflake writes them are removed.
fn array_remove(
    (array, value): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(ARRAY_SET_RULE);
    diverge(site.context, JSON_FIDELITY, construct)?;
    let reads = site.context.typed.reads();
    let (array, unordered) = free_variant(array, construct, site)?;
    let removed = convert_expression(value, site)?;
    let removed = match removed.data_type {
        SnowflakeType::SemiStructured if !removed.is_variant_held() => removed.sql,
        SnowflakeType::Null => "CAST(NULL AS JSON)".to_owned(),
        other => {
            return Err(refused(
                construct,
                &format!("ARRAY_REMOVE of a {other} value, which Snowflake rejects (error 1044)"),
            ));
        }
    };
    compared_by_text(site.context.typed.since(reads), construct, site.context)?;
    Ok(Emitted {
        unordered,
        ..semi_structured(json_array_remove((&array, &removed), log))
    })
}

/// ARRAY_APPEND: the array's elements then the value, as ARRAY_CAT with ARRAY_CONSTRUCT of it.
fn array_append(
    (array, value): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(ARRAY_APPEND_RULE);
    let (array, unordered) = free_variant(array, construct, site)?;
    let appended = array_construct(std::slice::from_ref(value), false, construct, site)?;
    if appended.is_variant_held() {
        return Err(refused(
            construct,
            "ARRAY_APPEND of a FLOAT, which DuckDB holds apart from JSON",
        ));
    }
    Ok(Emitted {
        unordered,
        ..semi_structured(json_array_cat((&array, &appended.sql), log))
    })
}

/// ARRAY_SLICE with whole-number bounds.
fn array_slice(
    (array, from, to): (&Value, &Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let array = variant(array, construct, site)?;
    let from = integer_operand(from, construct, site)?;
    let to = integer_operand(to, construct, site)?;
    Ok(semi_structured(json_array_slice(
        (&array, &from, &to),
        site.context.log,
    )))
}

/// ARRAY_GENERATE_RANGE with whole-number bounds and step, the step 1 when omitted.
fn array_range(
    (start, end, step): (&Value, &Value, Option<&Value>),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let start = integer_operand(start, construct, site)?;
    let end = integer_operand(end, construct, site)?;
    let step = match step {
        Some(step) => integer_operand(step, construct, site)?,
        None => "1".to_owned(),
    };
    Ok(semi_structured(json_array_range(
        (&start, &end, &step),
        site.context.log,
    )))
}
