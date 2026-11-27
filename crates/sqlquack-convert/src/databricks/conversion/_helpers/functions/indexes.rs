//! Functions taking a 1-based index, raising as Databricks does for an index of 0 or out of bounds.

use sqlquack_diagnostics::failures::constants::ARGUMENT_REJECTED;

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::functions::arrays::{element, known, small_index};
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::constants::{INDEX_BOUNDS_ERROR, INDEX_ZERO_ERROR};
use crate::databricks::conversion::models::{DatabricksType, Emitted};

/// The rewrite of the indexing function `name`, `None` when no rewrite handles it.
pub(crate) fn index_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "SPLIT_PART" => split_part,
        "ELEMENT_AT" => element_at,
        "TRY_ELEMENT_AT" => try_element_at,
        _ => return None,
    };
    Some(rewrite)
}

/// `SPLIT_PART(s, d, n)`: the `n`th part counting from the end when negative, empty out of bounds; an empty `d` splits nothing.
fn split_part(values: &[Emitted]) -> Option<Emitted> {
    let [text, delimiter, index] = values else {
        return None;
    };
    if text.data_type != DatabricksType::String || delimiter.data_type != DatabricksType::String {
        return None;
    }
    small_index(index)?;
    Some(typed(
        format!(
            "CASE WHEN {s} IS NULL OR {d} IS NULL OR {n} IS NULL THEN NULL \
             WHEN {n} = 0 THEN error('{ARGUMENT_REJECTED}: {INDEX_ZERO_ERROR}') \
             ELSE coalesce(list_extract(CASE WHEN {d} = '' THEN [{s}] ELSE string_split({s}, {d}) END, {n}), '') END",
            s = text.sql,
            d = delimiter.sql,
            n = index.sql
        ),
        DatabricksType::String,
    ))
}

fn element_at(values: &[Emitted]) -> Option<Emitted> {
    indexed(
        values,
        &format!("error('{ARGUMENT_REJECTED}: {INDEX_BOUNDS_ERROR}')"),
    )
}

fn try_element_at(values: &[Emitted]) -> Option<Emitted> {
    indexed(values, "NULL")
}

/// `ELEMENT_AT(a, i)` or `TRY_ELEMENT_AT(a, i)` of an array: raising for `i` of 0, `beyond` past either end.
fn indexed(values: &[Emitted], beyond: &str) -> Option<Emitted> {
    let [array, index] = values else { return None };
    let element = known(element(array)?)?;
    small_index(index)?;
    Some(typed(
        format!(
            "CASE WHEN {a} IS NULL OR {i} IS NULL THEN NULL \
             WHEN {i} = 0 THEN error('{ARGUMENT_REJECTED}: {INDEX_ZERO_ERROR}') \
             WHEN abs(CAST({i} AS BIGINT)) > len({a}) THEN {beyond} \
             ELSE list_extract({a}, {i}) END",
            a = array.sql,
            i = index.sql
        ),
        element.scalar(),
    ))
}
