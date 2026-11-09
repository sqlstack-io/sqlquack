//! `json_filter`: FILTER, the elements of an array a predicate keeps.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::lambdas::{elements, raising_non_arrays};
use crate::semi_structured::constants::JSON_FILTER;

/// The elements for which `predicate` over `parameter` is TRUE; non-arrays but objects raise.
pub fn json_filter(
    array: &str,
    (parameter, predicate): (&str, &str),
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_FILTER);
    raising_non_arrays(
        array,
        &format!(
            "CAST(to_json(list_filter({}, lambda {parameter}: {predicate})) AS JSON)",
            elements("q")
        ),
    )
}
