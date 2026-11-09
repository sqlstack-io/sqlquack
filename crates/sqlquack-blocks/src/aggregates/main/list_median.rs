//! `list_median`: MEDIAN, the middle value or the average of the two middle values.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::aggregates::constants::{LIST_MEDIAN, PERCENTILE_INTERPOLATION};
use crate::raises::main::raised::raised;

/// MEDIAN of `value` as `result_type`; with `bound`, middles at or beyond it raise.
pub fn list_median(
    value: &str,
    (result_type, bound): (&str, Option<&str>),
    log: &ProvenanceLog,
) -> String {
    log.building_block(LIST_MEDIAN);
    let low = "median_values[(len(median_values) + 1) // 2]";
    let high = "median_values[len(median_values) // 2 + 1]";
    let average = format!("(CAST({low} AS {result_type}) + CAST({high} AS {result_type})) * 0.5");
    let guard = match bound {
        Some(bound) => format!(
            "WHEN abs({low}) >= {bound} OR abs({high}) >= {bound} THEN CAST({} AS {result_type}) ",
            raised(&format!(
                "{PERCENTILE_INTERPOLATION}: a MEDIAN of values wider than recordings show \
                 Snowflake's double average keeping"
            ))
        ),
        None => String::new(),
    };
    format!(
        "list_transform([list_sort(list({value}) FILTER (WHERE {value} IS NOT NULL))], \
         lambda median_values: CASE WHEN coalesce(len(median_values), 0) = 0 THEN NULL {guard}\
         ELSE CAST({average} AS {result_type}) END)[1]"
    )
}
