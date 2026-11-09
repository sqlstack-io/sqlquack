//! `value_at_extreme`: MAX_BY and MIN_BY over rows with a key, raising where tied rows differ.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::aggregates::constants::{EXTREME_KEY_TIES, VALUE_AT_EXTREME};
use crate::raises::main::raised::raised;

/// The `value` at the extreme non-NULL `key`; tied rows holding different values raise, -0 beside 0.
pub fn value_at_extreme(
    (value, key): (&str, &str),
    largest: bool,
    (value_type, float): (&str, bool),
    log: &ProvenanceLog,
) -> String {
    log.building_block(VALUE_AT_EXTREME);
    let sign = match float {
        true => ", 's': signbit(top_row['v'])",
        false => "",
    };
    let extreme = match largest {
        true => "list_max",
        false => "list_min",
    };
    format!(
        "list_transform([list({{'k': {key}, 'v': {value}}}) FILTER (WHERE {key} IS NOT NULL)], \
         lambda keyed_rows: list_transform([list_filter(keyed_rows, lambda keyed_row: \
         keyed_row['k'] = {extreme}(list_transform(keyed_rows, lambda key_row: key_row['k'])))], \
         lambda top_rows: CASE WHEN coalesce(len(top_rows), 0) = 0 THEN NULL \
         WHEN len(list_distinct(list_transform(top_rows, lambda top_row: {{'v': top_row['v']{sign}}}))) = 1 \
         THEN top_rows[1]['v'] ELSE CAST({} AS {value_type}) END)[1])[1]",
        raised(&format!(
            "{EXTREME_KEY_TIES}: rows tied on the extreme key hold different values"
        ))
    )
}
