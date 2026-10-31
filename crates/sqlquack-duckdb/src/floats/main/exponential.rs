//! `exponential`: EXP as a double, raising where the result leaves the double range.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::floats::constants::EXPONENTIAL;
use crate::raises::main::raised::raised;

/// EXP of a value as a double; a result beyond the double range raises numeric_range.
pub fn exponential(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(EXPONENTIAL);
    let beyond =
        raised("numeric_range: an EXP beyond the range of a FLOAT, which no recording shows");
    format!(
        "list_transform([CAST({value} AS DOUBLE)], lambda x: CASE WHEN x > 0 AND isinf(exp(x)) \
         THEN CAST({beyond} AS DOUBLE) ELSE exp(x) END)[1]"
    )
}
