//! `json_reduce`: REDUCE, an array's elements folded left into a DOUBLE accumulator.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::semi_structured::_helpers::lambdas::elements;
use crate::semi_structured::constants::{JSON_FIDELITY, JSON_REDUCE};

/// `initial` folded left over the elements by `body`; NULL for NULL, and for a non-array from NULL.
pub fn json_reduce(
    array: &str,
    (accumulator, element, body): (&str, &str, &str),
    initial: &str,
    log: &ProvenanceLog,
) -> String {
    log.building_block(JSON_REDUCE);
    let other = raised(&format!(
        "{JSON_FIDELITY}: REDUCE of a value other than an array from a non-NULL initial value"
    ));
    format!(
        "list_transform([CAST({array} AS JSON)], lambda q: CASE WHEN q IS NULL THEN NULL \
         WHEN json_type(q) <> 'ARRAY' THEN CASE WHEN CAST({initial} AS DOUBLE) IS NULL THEN NULL \
         ELSE CAST({other} AS DOUBLE) END \
         ELSE list_reduce({}, lambda {accumulator}, {element}: CAST({body} AS DOUBLE), \
         CAST({initial} AS DOUBLE)) END)[1]",
        elements("q")
    )
}
