//! The two ways of summing FLOATs whose order of addition cannot change the result.

use crate::floats::constants::{DOMINANCE_MARGIN, EXACT_SUM_SCALES, EXACT_SUM_UNITS};

/// The sum from aggregates over `value`, which each repeats.
pub(crate) fn aggregated_sum(value: &str, distinct: &str, over: &str, raise: &str) -> String {
    let exact: Vec<String> = EXACT_SUM_SCALES
        .iter()
        .map(|scale| {
            let unit = 1_u64 << scale;
            format!(
                "(bool_and({value} * {unit} = trunc({value} * {unit})){over} \
                 AND sum(abs({value})){over} <= {})",
                EXACT_SUM_UNITS / unit
            )
        })
        .collect();
    let dominant = dominant(
        &format!("max(abs({value}), 2){over}"),
        &format!("count({value}){over}"),
    );
    format!(
        "(CASE WHEN count({value}){over} = 0 THEN NULL \
         WHEN bool_or(isnan({value})){over} THEN CAST('nan' AS DOUBLE) \
         WHEN count({value}){over} <= 2 OR {} OR {dominant} \
         THEN sum({distinct}{value}){over} \
         + 0.0 ELSE CAST({raise} AS DOUBLE) END)",
        exact.join(" OR "),
    )
}

/// The sum from one list of the values, so a long `value` is written once.
pub(crate) fn listed_sum(value: &str, distinct: bool, over: &str, raise: &str) -> String {
    let each = |body: &str| format!("list_transform(float_sum_values, lambda float_sum_x: {body})");
    let exact: Vec<String> = EXACT_SUM_SCALES
        .iter()
        .map(|scale| {
            let unit = 1_u64 << scale;
            format!(
                "(list_bool_and({}) AND list_sum({}) <= {})",
                each(&format!(
                    "float_sum_x * {unit} = trunc(float_sum_x * {unit})"
                )),
                each("abs(float_sum_x)"),
                EXACT_SUM_UNITS / unit
            )
        })
        .collect();
    let summed = match distinct {
        true => "list_distinct(float_sum_values)",
        false => "float_sum_values",
    };
    format!(
        "list_transform([list_filter(coalesce(list(CAST({value} AS DOUBLE)){over}, \
         CAST([] AS DOUBLE[])), lambda float_sum_x: float_sum_x IS NOT NULL)], \
         lambda float_sum_values: CASE WHEN len(float_sum_values) = 0 THEN NULL \
         WHEN list_bool_or({}) THEN CAST('nan' AS DOUBLE) \
         WHEN len(float_sum_values) <= 2 OR {} OR {} THEN list_sum({summed}) + 0.0 \
         ELSE CAST({raise} AS DOUBLE) END)[1]",
        each("isnan(float_sum_x)"),
        exact.join(" OR "),
        dominant(
            &format!("list_sort({}, 'DESC')[1:2]", each("abs(float_sum_x)")),
            "len(float_sum_values)"
        ),
    )
}

/// Whether one of `count` values, its magnitude first in `top`, absorbs every partial sum of the rest.
fn dominant(top: &str, count: &str) -> String {
    format!(
        "list_transform([{top}], lambda float_sum_top: len(float_sum_top) = 2 \
         AND isfinite(float_sum_top[1]) AND float_sum_top[1] > float_sum_top[2] \
         AND ({count} - 1) * float_sum_top[2] * {DOMINANCE_MARGIN} \
         < (float_sum_top[1] - nextafter(float_sum_top[1], 0)) / 2)[1]"
    )
}
