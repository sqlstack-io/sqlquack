//! Arrays: the `array(...)` constructor and the array functions whose NULLs and bounds DuckDB matches.

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::operators::{as_type, common_type};
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::_helpers::support::typing::wider_integer;
use crate::databricks::conversion::constants::LAMBDA_ELEMENT;
use crate::databricks::conversion::models::{DatabricksType, Element, Emitted};

/// The rewrite of the array function `name`, `None` when no rewrite handles it.
pub(crate) fn array_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "ARRAY" => construct,
        "SIZE" | "CARDINALITY" | "ARRAY_SIZE" => size,
        "ARRAY_MAX" => array_max,
        "ARRAY_MIN" => array_min,
        "GET" => get,
        "ARRAY_JOIN" => array_join,
        "ARRAYS_OVERLAP" => arrays_overlap,
        "ARRAY_APPEND" => array_append,
        "ARRAY_PREPEND" => array_prepend,
        "ARRAY_COMPACT" => array_compact,
        "REVERSE" => array_reverse,
        "SORT_ARRAY" => sort_array,
        "ARRAY_SORT" => array_sort,
        _ => return None,
    };
    Some(rewrite)
}

/// `array(a, b, ...)`: a DuckDB list of the arguments' common type; `array()` holds VOID.
fn construct(values: &[Emitted]) -> Option<Emitted> {
    let element = Element::of(common_type(values)?)?;
    let items: Vec<String> = values
        .iter()
        .map(|value| as_type(value, element.scalar()))
        .collect();
    Some(typed(
        format!("[{}]", items.join(", ")),
        DatabricksType::Array(element),
    ))
}

/// `SIZE`, `CARDINALITY` and `ARRAY_SIZE`: the element count as INT, NULL for a NULL array.
fn size(values: &[Emitted]) -> Option<Emitted> {
    let [array] = values else { return None };
    element(array)?;
    Some(typed(
        format!("CAST(len({}) AS INTEGER)", array.sql),
        DatabricksType::Int,
    ))
}

fn array_max(values: &[Emitted]) -> Option<Emitted> {
    extreme(values, "list_max")
}

fn array_min(values: &[Emitted]) -> Option<Emitted> {
    extreme(values, "list_min")
}

/// `ARRAY_MAX` or `ARRAY_MIN`: NULL elements skipped, NULL for an empty array, in both engines.
fn extreme(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [array] = values else { return None };
    let element = known(element(array)?)?;
    Some(typed(
        format!("{function}({})", array.sql),
        element.scalar(),
    ))
}

/// `GET(a, i)`: the element at the 0-based `i`, NULL out of bounds, where DuckDB counts negatives from the end.
fn get(values: &[Emitted]) -> Option<Emitted> {
    let [array, index] = values else { return None };
    let element = known(element(array)?)?;
    small_index(index)?;
    Some(typed(
        format!(
            "CASE WHEN {index} < 0 THEN NULL ELSE {array}[CAST({index} AS BIGINT) + 1] END",
            index = index.sql,
            array = array.sql
        ),
        element.scalar(),
    ))
}

/// `ARRAY_JOIN(a, d[, r])`: NULL elements skipped or replaced, NULL for a NULL `d` or `r`.
fn array_join(values: &[Emitted]) -> Option<Emitted> {
    let (array, delimiter, replacement) = match values {
        [array, delimiter] => (array, delimiter, None),
        [array, delimiter, replacement] => (array, delimiter, Some(replacement)),
        _ => return None,
    };
    let text = |value: &Emitted| {
        matches!(
            value.data_type,
            DatabricksType::String | DatabricksType::Null
        )
    };
    let strings = matches!(element(array)?, Element::String | Element::Null);
    if !strings || !text(delimiter) || !replacement.is_none_or(text) {
        return None;
    }
    let sql = match replacement {
        None => format!(
            "CASE WHEN {delimiter} IS NULL THEN NULL ELSE array_to_string({array}, {delimiter}) END",
            delimiter = delimiter.sql,
            array = array.sql
        ),
        Some(replacement) => format!(
            "CASE WHEN {delimiter} IS NULL OR {replacement} IS NULL THEN NULL \
             ELSE array_to_string(list_transform({array}, lambda {LAMBDA_ELEMENT}: \
             coalesce({LAMBDA_ELEMENT}, {replacement})), {delimiter}) END",
            delimiter = delimiter.sql,
            replacement = replacement.sql,
            array = array.sql
        ),
    };
    Some(typed(sql, DatabricksType::String))
}

/// `ARRAYS_OVERLAP(a, b)`: TRUE on a shared element, else NULL if both are non-empty and one holds NULL.
fn arrays_overlap(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = values else { return None };
    let (left_element, right_element) = (element(left)?, element(right)?);
    let common = match (left_element, right_element) {
        (left, right) if left == right => left,
        (Element::Null, other) | (other, Element::Null) => other,
        (left, right) => Element::of(wider_integer(left.scalar(), right.scalar())?)?,
    };
    let array = DatabricksType::Array(common);
    let (left, right) = (as_type(left, array), as_type(right, array));
    Some(typed(
        format!(
            "CASE WHEN {left} IS NULL OR {right} IS NULL THEN NULL \
             WHEN list_has_any({left}, {right}) THEN TRUE \
             WHEN len({left}) > 0 AND len({right}) > 0 AND \
             (list_position({left}, NULL) IS NOT NULL OR list_position({right}, NULL) IS NOT NULL) \
             THEN NULL ELSE FALSE END"
        ),
        DatabricksType::Boolean,
    ))
}

fn array_append(values: &[Emitted]) -> Option<Emitted> {
    added(values, |array, value| {
        format!("list_append({array}, {value})")
    })
}

fn array_prepend(values: &[Emitted]) -> Option<Emitted> {
    added(values, |array, value| {
        format!("list_prepend({value}, {array})")
    })
}

/// `ARRAY_APPEND` or `ARRAY_PREPEND`: NULL for a NULL array, where DuckDB starts a new list.
fn added(values: &[Emitted], list: fn(&str, &str) -> String) -> Option<Emitted> {
    let [array, value] = values else { return None };
    let element = known(element(array)?)?;
    let fits = value.data_type == DatabricksType::Null
        || wider_integer(element.scalar(), value.data_type) == Some(element.scalar())
        || value.data_type == element.scalar();
    if !fits {
        return None;
    }
    let value = as_type(value, element.scalar());
    Some(typed(
        format!(
            "CASE WHEN {array} IS NULL THEN NULL ELSE {} END",
            list(&array.sql, &value),
            array = array.sql
        ),
        array.data_type,
    ))
}

/// `ARRAY_COMPACT(a)`: the array without its NULL elements.
fn array_compact(values: &[Emitted]) -> Option<Emitted> {
    let [array] = values else { return None };
    element(array)?;
    Some(typed(
        format!(
            "list_filter({}, lambda {LAMBDA_ELEMENT}: {LAMBDA_ELEMENT} IS NOT NULL)",
            array.sql
        ),
        array.data_type,
    ))
}

/// `REVERSE` of an array; a string's reversal is left to a later rule.
fn array_reverse(values: &[Emitted]) -> Option<Emitted> {
    let [array] = values else { return None };
    element(array)?;
    Some(typed(
        format!("list_reverse({})", array.sql),
        array.data_type,
    ))
}

/// `SORT_ARRAY(a[, asc])`: NULLs first ascending, last descending, the order a constant chooses.
fn sort_array(values: &[Emitted]) -> Option<Emitted> {
    let (array, ascending) = match values {
        [array] => (array, true),
        [array, order] => match order.sql.as_str() {
            "TRUE" => (array, true),
            "FALSE" => (array, false),
            _ => return None,
        },
        _ => return None,
    };
    let order = match ascending {
        true => "'ASC', 'NULLS FIRST'",
        false => "'DESC', 'NULLS LAST'",
    };
    sorted(array, order)
}

/// `ARRAY_SORT(a)` without a comparator: ascending with NULLs last.
fn array_sort(values: &[Emitted]) -> Option<Emitted> {
    let [array] = values else { return None };
    sorted(array, "'ASC', 'NULLS LAST'")
}

fn sorted(array: &Emitted, order: &str) -> Option<Emitted> {
    known(element(array)?)?;
    Some(typed(
        format!("list_sort({}, {order})", array.sql),
        array.data_type,
    ))
}

/// The element type of an array argument, `None` for any other argument.
fn element(value: &Emitted) -> Option<Element> {
    match value.data_type {
        DatabricksType::Array(element) => Some(element),
        _ => None,
    }
}

/// An element type other than VOID, whose single values have a type.
fn known(element: Element) -> Option<Element> {
    match element {
        Element::Null => None,
        element => Some(element),
    }
}

/// An index Databricks takes as INT without a cast.
fn small_index(index: &Emitted) -> Option<()> {
    match index.data_type {
        DatabricksType::TinyInt | DatabricksType::SmallInt | DatabricksType::Int => Some(()),
        _ => None,
    }
}
