//! Sorting, flattening, appending, slicing and range functions over VARIANT arrays held as JSON.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::arrays::constants::ARRAY_SORT_ORDER;
use sqlquack_duckdb::arrays::main::json_array_flatten::json_array_flatten;
use sqlquack_duckdb::arrays::main::json_array_range::json_array_range;
use sqlquack_duckdb::arrays::main::json_array_slice::json_array_slice;
use sqlquack_duckdb::arrays::main::json_array_sort::json_array_sort;
use sqlquack_duckdb::arrays::main::json_to_array::json_to_array;
use sqlquack_duckdb::variants::main::json_array_cat::json_array_cat;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::objects::{refused, semi_structured, variant};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::text_functions::integer_operand;
use crate::conversion::_helpers::expressions::variants::array_construct;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::constants::{
    ARRAY_APPEND_RULE, ARRAY_FLATTEN_RULE, ARRAY_RANGE_RULE, ARRAY_SLICE_RULE, ARRAY_SORT_RULE,
    TO_ARRAY_RULE,
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
                .and_then(|()| variant(array, construct, site))
                .map(|array| semi_structured(json_array_sort(&array, log)))
        }
        ("ARRAY_FLATTEN", [array]) => {
            let _rule = log.rule(ARRAY_FLATTEN_RULE);
            variant(array, construct, site)
                .map(|array| semi_structured(json_array_flatten(&array, log)))
        }
        ("TO_ARRAY", [value]) => to_array(value, construct, site),
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

/// ARRAY_APPEND: the array's elements then the value, as ARRAY_CAT with ARRAY_CONSTRUCT of it.
fn array_append(
    (array, value): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(ARRAY_APPEND_RULE);
    let array = variant(array, construct, site)?;
    let appended = array_construct(std::slice::from_ref(value), false, construct, site)?;
    if appended.is_variant_held() {
        return Err(refused(
            construct,
            "ARRAY_APPEND of a FLOAT, which DuckDB holds apart from JSON",
        ));
    }
    Ok(semi_structured(json_array_cat(
        (&array, &appended.sql),
        log,
    )))
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
